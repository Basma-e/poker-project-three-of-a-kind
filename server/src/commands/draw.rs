//! Draw phase command handler
//! Handles DRAW command

use crate::state::ServerState;
use common::accounts::player::Player;
use poker::persistence::storage::save_player;
use poker::table::state::GamePhase;

/// Handle DRAW command
pub fn handle_draw_action(
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
        let mut engine_guard = engine.lock().unwrap();

        if table_guard.state.phase != GamePhase::Draw {
            return "ERROR Not in draw phase".to_string();
        }

        let seat = table_guard.players.iter().position(|tp| tp.id == p.id);

        if let Some(seat_idx) = seat {
            // Folded players cannot draw
            if seat_idx < table_guard.state.player_bet_states.len()
                && table_guard.state.player_bet_states[seat_idx].folded
            {
                return "ERROR You have folded; you cannot draw.".to_string();
            }

            // Enforce turn order in draw phase - only current actor can draw
            let current_actor_seat = table_guard.state.current_actor_seat;
            if current_actor_seat != seat_idx {
                // Check if current actor is disconnected - auto-skip their draw (draw none)
                if current_actor_seat < table_guard.players.len() {
                    let current_actor_id = table_guard.players[current_actor_seat].id;
                    let connected = server_state.connected_players.lock().unwrap();
                    if !connected.contains_key(&current_actor_id) {
                        drop(connected);
                        // Auto-draw none for disconnected player - add to pending and advance
                        let mut pending = server_state.pending_draws.lock().unwrap();
                        if !pending.contains_key(&current_actor_seat) {
                            pending.insert(current_actor_seat, Vec::new()); // Draw none
                        }
                        drop(pending);

                        // Advance to next active player
                        let player_count = table_guard.players.len();
                        let _advanced = table_guard.state.advance_to_next_actor(player_count);

                        // Check if all active players have submitted draws
                        let pending_guard = server_state.pending_draws.lock().unwrap();
                        let active_count = table_guard.state.active_player_count();
                        let submitted_count = pending_guard.len();
                        drop(pending_guard);

                        if submitted_count >= active_count {
                            // All draws submitted - process them
                            drop(engine_guard);
                            let mut engine_guard = engine.lock().unwrap();
                            let pending_guard = server_state.pending_draws.lock().unwrap();
                            let draws = pending_guard.clone();
                            drop(pending_guard);

                            if let Err(e) =
                                engine_guard.process_draw_phase(&mut *table_guard, draws)
                            {
                                return format!("ERROR {}", e);
                            }

                            // Clear pending draws
                            let mut pending = server_state.pending_draws.lock().unwrap();
                            pending.clear();
                        }

                        // Continue to next player
                        return "OK ACTION Disconnected player auto-skipped draw. Continue playing.".to_string();
                    }
                    drop(connected);
                }
                return "ERROR Not your turn to draw".to_string();
            }

            // Check if player has already drawn
            let mut pending = server_state.pending_draws.lock().unwrap();
            if pending.contains_key(&seat_idx) {
                return "ERROR You have already submitted your draw".to_string();
            }

            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.len() < 2 {
                return "ERROR Usage: DRAW <positions> or DRAW none".to_string();
            }

            let positions = if parts[1].to_lowercase() == "none" {
                Vec::new()
            } else {
                match parts[1..]
                    .iter()
                    .map(|s| s.parse::<usize>())
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(pos) => pos,
                    Err(_) => return "ERROR Invalid positions".to_string(),
                }
            };

            // Store this player's draw
            pending.insert(seat_idx, positions);
            *player = Some(table_guard.players[seat_idx].clone());

            // Advance to next active player
            let player_count = table_guard.players.len();
            let _advanced = table_guard.state.advance_to_next_actor(player_count);

            // Check if all active players have submitted draws
            let active_count = table_guard.state.active_player_count();
            if pending.len() >= active_count {
                // All players have submitted - process draws
                let draws = pending.clone();
                pending.clear();
                drop(pending);

                match engine_guard.process_draw_phase(&mut *table_guard, draws) {
                    Ok(_) => {
                        // Save updated players
                        for player_at_table in &table_guard.players {
                            let _ = save_player(player_at_table);
                        }

                        *player = Some(table_guard.players[seat_idx].clone());

                        // After draw, if more than one player, start second betting round
                        if table_guard.state.active_player_count() > 1 {
                            if table_guard.state.phase == GamePhase::BettingRound {
                                // Second betting round started
                                "OK DRAW All draws complete. Second betting round started!"
                                    .to_string()
                            } else {
                                "OK DRAW All draws complete".to_string()
                            }
                        } else {
                            engine_guard.transition_to_showdown(&mut *table_guard);
                            // Auto-determine winner if only one player
                            match engine_guard.determine_winner_and_award_pot(&mut *table_guard) {
                                Ok(_) => {
                                    // Reset table state for next hand
                                    table_guard.state.phase = GamePhase::WaitingForPlayers;
                                    table_guard.state.pot = 0;
                                    table_guard.state.current_bet_to_call = 0;
                                    engine_guard.reset();
                                    for player_at_table in &table_guard.players {
                                        let _ = save_player(player_at_table);
                                    }
                                    "OK DRAW All draws complete. Only one player left - going to showdown! Pot distributed. Ready for next hand!".to_string()
                                }
                                Err(e) => format!("ERROR {}", e),
                            }
                        }
                    }
                    Err(e) => format!("ERROR {}", e),
                }
            } else {
                // Waiting for other players - turn has been advanced
                format!(
                    "OK DRAW Draw recorded. Waiting for {} more player(s)...",
                    active_count - pending.len()
                )
            }
        } else {
            "ERROR Not at table".to_string()
        }
    } else {
        "ERROR No active game".to_string()
    }
}
