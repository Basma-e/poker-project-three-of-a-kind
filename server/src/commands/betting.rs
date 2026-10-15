//! Betting command handler
//! Handles BET, CALL, RAISE, FOLD, and CHECK commands

use crate::state::ServerState;
use common::accounts::player::Player;
use common::poker::variant::GameVariant;
use poker::betting::betting::BetAction;
use poker::persistence::storage::save_player;
use poker::table::state::GamePhase;

/// Handle betting actions (BET, CALL, RAISE, FOLD, CHECK)
pub fn handle_betting_action(
    command: &str,
    player: &mut Option<Player>,
    server_state: &ServerState,
) -> String {
    let p = match player.as_ref() {
        Some(p) => p,
        None => return "ERROR Not logged in".to_string(),
    };
    let table_opt = server_state.table.lock().unwrap();
    let engine_opt = server_state.game_engine.lock().unwrap();

    if let (Some(ref table), Some(ref engine)) = (table_opt.as_ref(), engine_opt.as_ref()) {
        let mut table_guard = table.lock().unwrap();
        let engine_guard = engine.lock().unwrap();

        let seat = table_guard.players.iter().position(|tp| tp.id == p.id);

        if let Some(seat_idx) = seat {
            if table_guard.state.phase != GamePhase::BettingRound {
                return "ERROR Not in betting round".to_string();
            }

            // Check if it's the current actor's turn, and if they're still connected
            let current_actor_seat = table_guard.state.current_actor_seat;
            if current_actor_seat != seat_idx {
                // Check if current actor is disconnected - auto-fold them
                if current_actor_seat < table_guard.players.len() {
                    let current_actor_id = table_guard.players[current_actor_seat].id;
                    let connected = server_state.connected_players.lock().unwrap();
                    if !connected.contains_key(&current_actor_id) {
                        drop(connected);
                        // Auto-fold disconnected player
                        drop(engine_guard);
                        let engine_guard = engine.lock().unwrap();
                        let _ =
                            engine_guard.process_betting_action(&mut *table_guard, BetAction::Fold);
                        // Continue to next player
                        return "OK ACTION Disconnected player auto-folded. Continue playing."
                            .to_string();
                    }
                    drop(connected);
                }
                return "ERROR Not your turn".to_string();
            }

            let parts: Vec<&str> = command.split_whitespace().collect();
            let action = match parts[0] {
                "BET" => {
                    if parts.len() < 2 {
                        return "ERROR Usage: BET <amount>".to_string();
                    }
                    match parts[1].parse::<i32>() {
                        Ok(amt) => BetAction::Bet(amt),
                        Err(_) => return "ERROR Invalid amount".to_string(),
                    }
                }
                "CALL" => BetAction::Call,
                "RAISE" => {
                    if parts.len() < 2 {
                        return "ERROR Usage: RAISE <amount>".to_string();
                    }
                    match parts[1].parse::<i32>() {
                        Ok(amt) => BetAction::Raise(amt),
                        Err(_) => return "ERROR Invalid amount".to_string(),
                    }
                }
                "FOLD" => BetAction::Fold,
                "CHECK" => BetAction::Check,
                _ => return "ERROR Invalid action".to_string(),
            };

            // Need mutable access to engine for process_betting_action
            drop(engine_guard);
            let mut engine_guard = engine.lock().unwrap();

            match engine_guard.process_betting_action(&mut *table_guard, action) {
                Ok(_) => {
                    // Update player in connected list
                    let mut connected = server_state.connected_players.lock().unwrap();
                    if let Some(connected_p) = connected.get_mut(&p.id) {
                        *connected_p = table_guard.players[seat_idx].clone();
                    }
                    // Also update all players at table
                    for player_at_table in &table_guard.players {
                        if let Some(connected_p) = connected.get_mut(&player_at_table.id) {
                            *connected_p = player_at_table.clone();
                        }
                    }
                    drop(connected);

                    // Save updated players
                    for player_at_table in &table_guard.players {
                        let _ = save_player(player_at_table);
                    }
                    //ensure to refresh the client's player copy
                    *player = Some(table_guard.players[seat_idx].clone());

                    if table_guard.state.is_betting_round_complete() {
                        // Auto-transition based on game phase
                        if table_guard.state.phase == GamePhase::BettingRound {
                            if table_guard.state.active_player_count() <= 1 {
                                engine_guard.transition_to_showdown(&mut *table_guard);
                                // Process showdown when only one player remains
                                match engine_guard.determine_winner_and_award_pot(&mut *table_guard)
                                {
                                    Ok(winners) => {
                                        // Update all players in connected list
                                        let mut connected =
                                            server_state.connected_players.lock().unwrap();
                                        for player_at_table in &table_guard.players {
                                            if let Some(connected_p) =
                                                connected.get_mut(&player_at_table.id)
                                            {
                                                *connected_p = player_at_table.clone();
                                            }
                                            let _ = save_player(player_at_table);
                                        }
                                        drop(connected);

                                        // Build showdown result message
                                        let winner_names: Vec<String> = winners
                                            .iter()
                                            .map(|&seat| table_guard.players[seat].name.clone())
                                            .collect();

                                        // Get the winner's hand for display
                                        let winner_seat = winners[0];
                                        let mut all_hands_info = Vec::new();
                                        if let Some(cards) =
                                            engine_guard.get_player_cards(&table_guard, winner_seat)
                                        {
                                            let cards_str: Vec<String> =
                                                cards.iter().map(|c| c.to_string()).collect();
                                            if let Ok(Some(eval)) = engine_guard
                                                .evaluate_current_player_hand(
                                                    &table_guard,
                                                    winner_seat,
                                                )
                                            {
                                                all_hands_info.push(format!(
                                                    "{}: {} - {:?}",
                                                    table_guard.players[winner_seat].name,
                                                    cards_str.join(" "),
                                                    eval.rank
                                                ));
                                            } else {
                                                all_hands_info.push(format!(
                                                    "{}: {}",
                                                    table_guard.players[winner_seat].name,
                                                    cards_str.join(" ")
                                                ));
                                            }
                                        }

                                        let showdown_result = format!(
                                            "OK SHOWDOWN All hands: {} | Winners: {} | Pot distributed. Ready for next hand!",
                                            all_hands_info.join(" | "),
                                            winner_names.join(", ")
                                        );

                                        // Store showdown results so all players can see them
                                        {
                                            let mut last_showdown =
                                                server_state.last_showdown.lock().unwrap();
                                            *last_showdown = Some(showdown_result.clone());
                                        }

                                        // Reset for next hand
                                        table_guard.state.phase = GamePhase::WaitingForPlayers;
                                        table_guard.state.pot = 0;
                                        table_guard.state.current_bet_to_call = 0;
                                        engine_guard.reset();
                                    }
                                    Err(e) => {
                                        eprintln!("Error determining winner: {}", e);
                                    }
                                }
                            } else {
                                match table_guard.variant {
                                    GameVariant::FiveCardDraw => {
                                        let has_been_through_draw =
                                            engine_guard.has_been_through_draw();
                                        if has_been_through_draw {
                                            engine_guard.transition_to_showdown(&mut *table_guard);
                                        } else {
                                            engine_guard.transition_to_draw(&mut *table_guard);
                                            let mut pending =
                                                server_state.pending_draws.lock().unwrap();
                                            pending.clear();
                                        }
                                    }
                                    GameVariant::TexasHoldem => {
                                        if let Err(e) = engine_guard
                                            .advance_holdem_after_betting_round(&mut *table_guard)
                                        {
                                            return format!("ERROR {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        "OK ACTION Betting round complete!".to_string()
                    } else {
                        "OK ACTION Action processed".to_string()
                    }
                }
                Err(e) => format!("ERROR {}", e),
            }
        } else {
            "ERROR Not at table".to_string()
        }
    } else {
        "ERROR No active game".to_string()
    }
}
