//! Table management command handlers
//! Handles JOIN_TABLE and START_GAME commands

use crate::state::ServerState;
use common::accounts::player::Player;
use common::poker::variant::GameVariant;
use poker::game::GameEngine;
use poker::persistence::storage::save_player;
use poker::rules::rules::HouseRules;
use poker::table::state::GamePhase;
use poker::table::table::Table;
use std::sync::{Arc, Mutex};

fn lobby_dealer_seat(table: &Table) -> Option<usize> {
    let n = table.players.len();
    if n == 0 {
        None
    } else if table.state.phase == GamePhase::WaitingForPlayers {
        Some((table.state.dealer_seat + 1) % n)
    } else {
        Some(table.state.dealer_seat)
    }
}

/// Handle JOIN_TABLE command
pub fn handle_join_table(player: &Option<Player>, server_state: &ServerState) -> String {
    if let Some(p) = player.as_ref() {
        // Get or create table
        let table = {
            let mut table_opt = server_state.table.lock().unwrap();

            if table_opt.is_none() {
                let new_table = Arc::new(Mutex::new(Table::new()));
                let new_engine = Arc::new(Mutex::new(GameEngine::new(HouseRules::default_rules())));
                let table_clone = Arc::clone(&new_table);
                *table_opt = Some(new_table);
                let mut engine_opt = server_state.game_engine.lock().unwrap();
                *engine_opt = Some(new_engine);
                table_clone
            } else {
                Arc::clone(table_opt.as_ref().unwrap())
            }
        };

        let mut table_guard = table.lock().unwrap();

        // Check if player already at table
        if table_guard.players.iter().any(|tp| tp.id == p.id) {
            let phase = format!("{:?}", table_guard.state.phase);
            if table_guard.state.phase == GamePhase::WaitingForPlayers {
                return "OK JOIN Already at table".to_string();
            } else {
                // Player is already at table, game in progress - they can continue
                return format!(
                    "OK JOIN Already at table. Game phase: {}. Continue playing.",
                    phase
                );
            }
        }

        // Check if game is in progress - don't allow new players to join mid-hand
        let game_in_progress = table_guard.state.phase != GamePhase::WaitingForPlayers;
        if game_in_progress {
            return "ERROR Cannot join table while game is in progress. Wait for current hand to finish.".to_string();
        }

        // Add player to table
        match table_guard.add_player(p.clone()) {
            Ok(_) => {
                let player_count = table_guard.players.len();
                // Save updated player list
                for player_at_table in &table_guard.players {
                    let _ = save_player(player_at_table);
                }

                if table_guard.can_start() {
                    format!(
                        "OK JOIN Joined table. {} players at table. Ready to start! Use START_GAME when ready.",
                        player_count
                    )
                } else {
                    format!(
                        "OK JOIN Joined table. {} players at table. Waiting for more players...",
                        player_count
                    )
                }
            }
            Err(e) => format!("ERROR {}", e),
        }
    } else {
        "ERROR Not logged in".to_string()
    }
}

/// Handle START_GAME command
pub fn handle_start_game(player: &Option<Player>, server_state: &ServerState) -> String {
    if let Some(p) = player.as_ref() {
        let table_opt = server_state.table.lock().unwrap();
        let engine_opt = server_state.game_engine.lock().unwrap();

        if let (Some(ref table), Some(ref engine)) = (table_opt.as_ref(), engine_opt.as_ref()) {
            let mut table_guard = table.lock().unwrap();
            let mut engine_guard = engine.lock().unwrap();

            // Check if player is at table
            let seat = table_guard.players.iter().position(|tp| tp.id == p.id);
            if seat.is_none() {
                return "ERROR Not at table. Use JOIN_TABLE first".to_string();
            }
            let seat = seat.unwrap();

            // Dealer controls table start (dealer's choice flow).
            let dealer_seat = lobby_dealer_seat(&table_guard).unwrap_or(0);
            if seat != dealer_seat {
                return format!(
                    "ERROR Only dealer can start game. Current dealer: seat {}",
                    dealer_seat
                );
            }

            if !table_guard.can_start() {
                return "ERROR Not enough players (need 2-5)".to_string();
            }
            if !table_guard.variant_locked {
                return "ERROR Dealer must select variant first (SET_VARIANT DRAW|HOLDEM)"
                    .to_string();
            }

            // If game is already in progress, allow player to join ongoing game
            if table_guard.state.phase != GamePhase::WaitingForPlayers {
                // Game is in progress - player can join and continue playing
                let phase = format!("{:?}", table_guard.state.phase);
                return format!(
                    "OK GAME_IN_PROGRESS Game already in progress (phase: {}). You can continue playing.",
                    phase
                );
            }

            match engine_guard.start_hand(&mut *table_guard) {
                Ok(_) => {
                    // Clear previous showdown results when starting new hand
                    {
                        let mut last_showdown = server_state.last_showdown.lock().unwrap();
                        *last_showdown = None;
                    }
                    let msg = match table_guard.variant {
                        GameVariant::FiveCardDraw => {
                            let ante_amount = engine_guard.rules.ante_amount;
                            let player_count = table_guard.players.len();
                            let total_ante = ante_amount * player_count as i32;
                            if ante_amount > 0 {
                                format!(
                                    "Variant: {} | Ante collected: {} from each player (total: {})",
                                    table_guard.variant.display_name(),
                                    ante_amount,
                                    total_ante
                                )
                            } else {
                                format!("Variant: {} | No ante", table_guard.variant.display_name())
                            }
                        }
                        GameVariant::TexasHoldem => format!(
                            "Variant: {} | Blinds posted: SB {} / BB {}",
                            table_guard.variant.display_name(),
                            engine_guard.rules.small_blind,
                            engine_guard.rules.big_blind
                        ),
                    };
                    format!(
                        "OK GAME_STARTED {} | Dealer: seat {}, First to act: seat {}",
                        msg, table_guard.state.dealer_seat, table_guard.state.current_actor_seat
                    )
                }
                Err(e) => format!("ERROR {}", e),
            }
        } else {
            "ERROR No table available".to_string()
        }
    } else {
        "ERROR Not logged in".to_string()
    }
}

/// Handle SET_VARIANT command
pub fn handle_set_variant(
    parts: &[&str],
    player: &Option<Player>,
    server_state: &ServerState,
) -> String {
    if player.is_none() {
        return "ERROR Not logged in".to_string();
    }
    if parts.len() < 2 {
        return "ERROR Usage: SET_VARIANT <DRAW|HOLDEM>".to_string();
    }
    let variant = match GameVariant::from_token(parts[1]) {
        Some(v) => v,
        None => return "ERROR Unknown variant. Use DRAW or HOLDEM".to_string(),
    };

    let table_opt = server_state.table.lock().unwrap();
    if let Some(ref table) = table_opt.as_ref() {
        let mut table_guard = table.lock().unwrap();
        let p = player.as_ref().unwrap();
        let seat = match table_guard.players.iter().position(|tp| tp.id == p.id) {
            Some(s) => s,
            None => return "ERROR Not at table. Use JOIN_TABLE first".to_string(),
        };
        if table_guard.state.phase != GamePhase::WaitingForPlayers {
            return "ERROR Cannot change variant while a hand is in progress".to_string();
        }
        let dealer_seat = lobby_dealer_seat(&table_guard).unwrap_or(0);
        if seat != dealer_seat {
            return format!(
                "ERROR Only dealer can choose variant. Current dealer: seat {}",
                dealer_seat
            );
        }

        if table_guard.variant_locked {
            return format!(
                "ERROR Variant already locked as {}",
                table_guard.variant.display_name()
            );
        }
        
        table_guard.variant = variant;
        table_guard.variant_locked = true;
        format!("OK VARIANT {}", variant.display_name())
    } else {
        "ERROR No active table".to_string()
    }
}

/// Handle GET_TABLE_INFO command
pub fn handle_get_table_info(player: &Option<Player>, server_state: &ServerState) -> String {
    if let Some(p) = player.as_ref() {
        let table_opt = server_state.table.lock().unwrap();
        let engine_opt = server_state.game_engine.lock().unwrap();
        if let Some(ref table) = table_opt.as_ref() {
            let table_guard = table.lock().unwrap();
            let engine_guard = if let Some(ref engine) = engine_opt.as_ref() {
                Some(engine.lock().unwrap())
            } else {
                None
            };

            // Find player's seat
            let player_seat = table_guard.players.iter().position(|tp| tp.id == p.id);

            if let Some(_seat_idx) = player_seat {
                let mut seat_info = Vec::new();
                let player_count = table_guard.players.len();
                // Before the hand starts, dealer_seat is from the last hand; show who will be dealer next
                let dealer_seat_to_show = if table_guard.state.phase == GamePhase::WaitingForPlayers
                    && player_count > 0
                {
                    (table_guard.state.dealer_seat + 1) % player_count
                } else {
                    table_guard.state.dealer_seat
                };
                let (small_blind_seat, big_blind_seat) = if player_count > 0 {
                    (
                        (table_guard.state.dealer_seat + 1) % player_count,
                        (table_guard.state.dealer_seat + 2) % player_count,
                    )
                } else {
                    (0, 0)
                };
                let holdem_preflop = table_guard.variant == GameVariant::TexasHoldem
                    && engine_guard
                        .as_ref()
                        .and_then(|e| e.holdem_street())
                        .map(|s| format!("{:?}", s) == "PreFlop")
                        .unwrap_or(false);

                for (seat, player_at_seat) in table_guard.players.iter().enumerate() {
                    let mut markers = Vec::new();

                    // Mark dealer (for next hand when waiting, or current dealer during a hand)
                    if seat == dealer_seat_to_show {
                        markers.push("[D]");
                    }
                    if holdem_preflop && seat == small_blind_seat {
                        markers.push("[SB]");
                    }
                    if holdem_preflop && seat == big_blind_seat {
                        markers.push("[BB]");
                    }

                    // Mark current actor
                    if (table_guard.state.phase == GamePhase::BettingRound
                        || table_guard.state.phase == GamePhase::Draw)
                        && seat == table_guard.state.current_actor_seat
                    {
                        markers.push("[*]");
                    }

                    // Mark requesting player
                    if Some(seat) == player_seat {
                        markers.push("[YOU]");
                    }

                    // Mark folded players
                    if seat < table_guard.state.player_bet_states.len()
                        && table_guard.state.player_bet_states[seat].folded
                    {
                        markers.push("[F]");
                    }

                    let markers_str = if markers.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", markers.join(" "))
                    };

                    seat_info.push(format!(
                        "Seat {}: {}{}",
                        seat, player_at_seat.name, markers_str
                    ));
                }

                let pot = table_guard.state.pot;
                let bet_to_call = table_guard.state.current_bet_to_call;
                let blinds_info = if table_guard.variant == GameVariant::TexasHoldem {
                    if let Some(engine) = engine_guard.as_ref() {
                        format!(
                            " | Blinds: SB seat {} ({}) / BB seat {} ({})",
                            small_blind_seat,
                            engine.rules.small_blind,
                            big_blind_seat,
                            engine.rules.big_blind
                        )
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                format!(
                    "OK TABLE_INFO Table ID: 1 | {} | Variant: {}{} | Pot: {} | Bet to call: {} | Phase: {:?} | Dealer: seat {}",
                    seat_info.join(" | "),
                    table_guard.variant.display_name(),
                    blinds_info,
                    pot,
                    bet_to_call,
                    table_guard.state.phase,
                    dealer_seat_to_show
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

/// Handle LEAVE_TABLE command
pub fn handle_leave_table(player: &Option<Player>, server_state: &ServerState) -> String {
    if let Some(p) = player.as_ref() {
        let table_opt = server_state.table.lock().unwrap();
        if let Some(ref table) = table_opt.as_ref() {
            let mut table_guard = table.lock().unwrap();

            // Only allow leaving when game is waiting for players
            if table_guard.state.phase != GamePhase::WaitingForPlayers {
                return "ERROR Cannot leave table while game is in progress. Wait for current hand to finish.".to_string();
            }

            // Find and remove player from table
            let initial_len = table_guard.players.len();
            table_guard.players.retain(|tp| tp.id != p.id);

            if table_guard.players.len() < initial_len {
                // Player was removed, save all remaining players
                for player_at_table in &table_guard.players {
                    let _ = save_player(player_at_table);
                }

                // Also remove from connected players
                let mut connected = server_state.connected_players.lock().unwrap();
                connected.remove(&p.id);

                // If table is empty, unlock variant selection for next lobby.
                if table_guard.players.is_empty() {
                    table_guard.variant = GameVariant::FiveCardDraw;
                    table_guard.variant_locked = false;
                }

                format!(
                    "OK LEFT_TABLE Left the table. {} players remaining.",
                    table_guard.players.len()
                )
            } else {
                "OK LEFT_TABLE Not at table".to_string()
            }
        } else {
            "OK LEFT_TABLE Not at table".to_string()
        }
    } else {
        "ERROR Not logged in".to_string()
    }
}
