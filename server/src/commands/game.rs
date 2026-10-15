//! Game state command handlers
//! Handles GET_GAME_STATE, GET_HAND, and SHOWDOWN commands

use crate::state::ServerState;
use common::accounts::player::Player;
use common::poker::variant::GameVariant;
use poker::persistence::storage::save_player;
use poker::table::state::GamePhase;

/// Handle GET_GAME_STATE command
pub fn handle_get_game_state(player: &Option<Player>, server_state: &ServerState) -> String {
    if let Some(p) = player.as_ref() {
        let table_opt = server_state.table.lock().unwrap();
        let engine_opt = server_state.game_engine.lock().unwrap();

        if let (Some(ref table), Some(ref engine)) = (table_opt.as_ref(), engine_opt.as_ref()) {
            let table_guard = table.lock().unwrap();
            let engine_guard = engine.lock().unwrap();

            // Find player's seat
            let seat = table_guard.players.iter().position(|tp| tp.id == p.id);

            if let Some(seat_idx) = seat {
                // Return "Your turn: true" during BettingRound or Draw when it's this player's turn.
                // Folded players never get "your turn" in Draw (or BettingRound).
                let is_folded = seat_idx < table_guard.state.player_bet_states.len()
                    && table_guard.state.player_bet_states[seat_idx].folded;
                let is_my_turn = !is_folded
                    && (table_guard.state.phase == GamePhase::BettingRound
                        || table_guard.state.phase == GamePhase::Draw)
                    && table_guard.state.current_actor_seat == seat_idx;
                let phase = format!("{:?}", table_guard.state.phase);
                let pot = table_guard.state.pot;
                let bet_to_call = table_guard.state.current_bet_to_call;

                // Get ante amount from engine rules for display
                let ante_amount = engine_guard.rules.ante_amount;
                let variant = table_guard.variant.display_name();
                let street = engine_guard
                    .holdem_street()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "N/A".to_string());
                let board = if table_guard.variant == GameVariant::TexasHoldem {
                    engine_guard
                        .holdem_community_cards()
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    "N/A".to_string()
                };
                let blinds = if table_guard.variant == GameVariant::TexasHoldem {
                    let player_count = table_guard.players.len();
                    if player_count > 0 {
                        let sb = (table_guard.state.dealer_seat + 1) % player_count;
                        let bb = (table_guard.state.dealer_seat + 2) % player_count;
                        format!(
                            "SB seat {} ({}) / BB seat {} ({})",
                            sb, engine_guard.rules.small_blind, bb, engine_guard.rules.big_blind
                        )
                    } else {
                        "N/A".to_string()
                    }
                } else {
                    "N/A".to_string()
                };

                // Calculate current actor and next actor info
                let (current_actor_name, next_actor_info, players_until_turn) =
                    if table_guard.state.phase == GamePhase::BettingRound
                        || table_guard.state.phase == GamePhase::Draw
                    {
                        let current_actor_seat = table_guard.state.current_actor_seat;
                        let current_actor_name = if current_actor_seat < table_guard.players.len() {
                            table_guard.players[current_actor_seat].name.clone()
                        } else {
                            "Unknown".to_string()
                        };

                        // Calculate next actor and players until player's turn
                        let player_count = table_guard.players.len();
                    //     let mut next_seat = (current_actor_seat + 1) % player_count;
                    //     let mut players_until = 0;
                    //     let mut found_next = false;
                    //     let mut next_actor_name = String::new();

                    //     // Find next active player
                    //     while next_seat != seat_idx && players_until < player_count {
                    //         if next_seat < table_guard.state.player_bet_states.len()
                    //             && !table_guard.state.player_bet_states[next_seat].folded
                    //         {
                    //             if !found_next {
                    //                 next_actor_name = table_guard.players[next_seat].name.clone();
                    //                 found_next = true;
                    //             }
                    //             players_until += 1;
                    //         }
                    //         next_seat = (next_seat + 1) % player_count;
                    //     }

                    //     let next_info = if is_my_turn {
                    //         "You".to_string()
                    //     } else if found_next && players_until == 0 {
                    //         "You".to_string()
                    //     } else if found_next {
                    //         format!("{} (in {} player(s))", next_actor_name, players_until)
                    //     } else {
                    //         "Unknown".to_string()
                    //     };

                    //     (current_actor_name, next_info, players_until)
                    // } else {
                    //     ("N/A".to_string(), "N/A".to_string(), 0)
                    // };
                    let mut next_actor_name = "N/A".to_string();
                    let mut players_until_turn = 0usize;

                    if player_count > 0 {
                        for step in 1..=player_count {
                            let candidate = (current_actor_seat + step) % player_count;

                            let folded = candidate < table_guard.state.player_bet_states.len()
                                && table_guard.state.player_bet_states[candidate].folded;

                            if !folded {
                                next_actor_name = table_guard.players[candidate].name.clone();

                                if !is_my_turn {
                                    let mut count = 0usize;
                                    for inner_step in 1..=player_count {
                                        let between = (current_actor_seat + inner_step) % player_count;

                                        let between_folded =
                                            between < table_guard.state.player_bet_states.len()
                                                && table_guard.state.player_bet_states[between].folded;

                                        if !between_folded {
                                            if between == seat_idx {
                                                break;
                                            }
                                            count += 1;
                                        }
                                    }
                                    players_until_turn = count;
                                }
                                break;
                            }
                        }
                    }

                    (current_actor_name, next_actor_name, players_until_turn)
                } else {
                    ("N/A".to_string(), "N/A".to_string(), 0usize)
                };


                format!(
                    "OK GAME_STATE Variant: {} | Street: {} | Board: {} | Blinds: {} | Phase: {} | Pot: {} | Bet to call: {} | Your turn: {} | Current actor: {} | Next: {} | Until your turn: {} | Ante: {}",
                    variant,
                    street,
                    board,
                    blinds,
                    phase,
                    pot,
                    bet_to_call,
                    is_my_turn,
                    current_actor_name,
                    next_actor_info,
                    players_until_turn,
                    ante_amount
                )
            } else {
                "ERROR Not at table".to_string()
            }
        } else {
            "ERROR No active game".to_string()
        }
    } else {
        "ERROR Not logged in".to_string()
    }
}

/// Handle GET_HAND command
pub fn handle_get_hand(player: &Option<Player>, server_state: &ServerState) -> String {
    if let Some(p) = player.as_ref() {
        let table_opt = server_state.table.lock().unwrap();
        let engine_opt = server_state.game_engine.lock().unwrap();

        if let (Some(ref table), Some(ref engine)) = (table_opt.as_ref(), engine_opt.as_ref()) {
            let table_guard = table.lock().unwrap();
            let engine_guard = engine.lock().unwrap();

            let seat = table_guard.players.iter().position(|tp| tp.id == p.id);

            if let Some(seat_idx) = seat {
                if let Some(cards) = engine_guard.get_player_cards(&table_guard, seat_idx) {
                    let cards_str: Vec<String> = cards.iter().map(|c| c.to_string()).collect();
                    format!("OK HAND {}", cards_str.join(" "))
                } else {
                    "ERROR No hand available".to_string()
                }
            } else {
                "ERROR Not at table".to_string()
            }
        } else {
            "ERROR No active game".to_string()
        }
    } else {
        "ERROR Not logged in".to_string()
    }
}

/// Handle SHOWDOWN command
pub fn handle_showdown(player: &mut Option<Player>, server_state: &ServerState) -> String {
    if player.is_none() {
        return "ERROR Not logged in".to_string();
    }

    let table_opt = server_state.table.lock().unwrap();
    let engine_opt = server_state.game_engine.lock().unwrap();

    if let (Some(ref table), Some(ref engine)) = (table_opt.as_ref(), engine_opt.as_ref()) {
        let mut table_guard = table.lock().unwrap();
        let mut engine_guard = engine.lock().unwrap();

        // if table_guard.state.phase != GamePhase::Showdown {
        //     // If showdown was already processed, return stored results
        //     if table_guard.state.phase == GamePhase::WaitingForPlayers {
        //         let last_showdown = server_state.last_showdown.lock().unwrap();
        //         if let Some(ref result) = *last_showdown {
        //             return result.clone();
        //         }
        //         return "OK SHOWDOWN Already processed. Game reset for next hand.".to_string();
        //     }
        //     return "ERROR Not in showdown phase".to_string();
        // }
        if table_guard.state.phase != GamePhase::Showdown {
            if table_guard.state.phase == GamePhase::WaitingForPlayers {
                // Refresh local player even if showdown was already processed
                if let Some(p0) = player.as_ref() {
                    // best source is connected_players (should be up-to-date)
                    let connected = server_state.connected_players.lock().unwrap();
                    if let Some(updated) = connected.get(&p0.id) {
                        *player = Some(updated.clone());
                    }
                }

                let last_showdown = server_state.last_showdown.lock().unwrap();
                if let Some(ref result) = *last_showdown {
                    return result.clone();
                }
                return "OK SHOWDOWN Already processed. Game reset for next hand.".to_string();
            }
            return "ERROR Not in showdown phase".to_string();
        }

        // Get all player hands and evaluations for display BEFORE processing
        let mut all_hands_info = Vec::new();
        for (seat, player) in table_guard.players.iter().enumerate() {
            if seat < table_guard.state.player_bet_states.len()
                && !table_guard.state.player_bet_states[seat].folded
            {
                if let Some(cards) = engine_guard.get_player_cards(&table_guard, seat) {
                    if let Ok(Some(eval)) =
                        engine_guard.evaluate_current_player_hand(&table_guard, seat)
                    {
                        let cards_str: Vec<String> = cards.iter().map(|c| c.to_string()).collect();
                        all_hands_info.push(format!(
                            "{}: {} - {:?}",
                            player.name,
                            cards_str.join(" "),
                            eval.rank
                        ));
                    } else {
                        let cards_str: Vec<String> = cards.iter().map(|c| c.to_string()).collect();
                        all_hands_info.push(format!("{}: {}", player.name, cards_str.join(" ")));
                    }
                }
            }
        }

        // Determine winner (only process once, but allow multiple clients to trigger)
        match engine_guard.determine_winner_and_award_pot(&mut *table_guard) {
            Ok(winners) => {
                // Update all players in connected list
                let mut connected = server_state.connected_players.lock().unwrap();
                for player_at_table in &table_guard.players {
                    if let Some(connected_p) = connected.get_mut(&player_at_table.id) {
                        *connected_p = player_at_table.clone();
                    }
                    let _ = save_player(player_at_table);
                }
                drop(connected);

                let winner_names: Vec<String> = winners
                    .iter()
                    .map(|&seat| table_guard.players[seat].name.clone())
                    .collect();

                // Build showdown result message
                let showdown_result = format!(
                    "OK SHOWDOWN All hands: {} | Winners: {} | Pot distributed. Ready for next hand!",
                    all_hands_info.join(" | "),
                    winner_names.join(", ")
                );

                // Store showdown results so all players can see them even after state reset
                {
                    let mut last_showdown = server_state.last_showdown.lock().unwrap();
                    *last_showdown = Some(showdown_result.clone());
                }

                // Reset game state for next hand
                table_guard.state.phase = GamePhase::WaitingForPlayers;
                table_guard.state.pot = 0;
                table_guard.state.current_bet_to_call = 0;
                engine_guard.reset();

                // refresh local player so menu shows updated balance/stats/history
                if let Some(p0) = player.as_ref() {
                    if let Some(updated) = table_guard.players.iter().find(|tp| tp.id == p0.id) {
                        *player = Some(updated.clone());
                    }
                }

                showdown_result
            }
            Err(e) => format!("ERROR {}", e),
        }
    } else {
        "ERROR No active game".to_string()
    }
}
