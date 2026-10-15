//! Gameplay module for client
//! Handles the main game playing logic

use crate::input::get_input;
use crate::network::ClientConnection;
use std::io::{self, Write};

/// Main game playing function
pub fn play_game(conn: &mut ClientConnection) {
    println!("\n=== Play Game ===");
    
    // Join table
    conn.send_command("JOIN_TABLE");
    let response = conn.read_response();
    println!("{}", response);
    
    if !response.starts_with("OK") {
        return;
    }

    choose_variant(conn);

    // Main game loop - can play multiple hands
    loop {
        // Wait for game to start or start it (displays table layout)
        // If player left, return to main menu
        if !wait_for_game_start(conn) {
            return;
        }

        // Game loop - play one hand
        let mut previous_state: Option<String> = None;
        let mut ante_shown = false; // Track if ante has been shown for this hand
        loop {
            conn.send_command("GET_GAME_STATE");
            let state = conn.read_response();
            
            // Check if game just started (transition from WaitingForPlayers to BettingRound)
            // Show ante information for all players when game starts
            if !ante_shown && state.contains("BettingRound") {
                // Extract ante info from game state
                if let Some(ante_part) = state.split("Ante: ").nth(1) {
                    if let Some(ante_str) = ante_part.split(' ').next() {
                        if let Ok(ante_amount) = ante_str.parse::<i32>() {
                            if ante_amount > 0 {
                                // Get player count to calculate total
                                conn.send_command("GET_TABLE_INFO");
                                let table_info = conn.read_response();
                                let player_count = if table_info.contains("Seat") {
                                    table_info.matches("Seat").count()
                                } else {
                                    0
                                };
                                let total_ante = ante_amount * player_count as i32;
                                println!("\n💰 Ante collected: ${} from each player (total: ${})", ante_amount, total_ante);
                                ante_shown = true;
                            }
                        }
                    }
                }
            }
            
            // Check if game has already been reset (showdown already processed)
            if state.contains("WaitingForPlayers") {
                // Try to get showdown results if available
                conn.send_command("SHOWDOWN");
                let showdown_result = conn.read_response();
                if showdown_result.starts_with("OK SHOWDOWN") {
                    // Format showdown nicely
                    format_showdown_result(&showdown_result);
                }
                // Game was reset, showdown already processed
                break;
            }
            
            // Check if game just started (transition from WaitingForPlayers to BettingRound)
            // Show ante information for all players when game starts
            if state.contains("BettingRound") && previous_state.as_ref().map(|s| s.contains("WaitingForPlayers")).unwrap_or(false) {
                // Extract ante info from game state
                if let Some(ante_part) = state.split("Ante: ").nth(1) {
                    if let Some(ante_str) = ante_part.split(' ').next() {
                        if let Ok(ante_amount) = ante_str.parse::<i32>() {
                            if ante_amount > 0 {
                                // Get player count to calculate total
                                conn.send_command("GET_TABLE_INFO");
                                let table_info = conn.read_response();
                                let player_count = if table_info.contains("Seat") {
                                    table_info.matches("Seat").count()
                                } else {
                                    0
                                };
                                let total_ante = ante_amount * player_count as i32;
                                println!("\n💰 Ante collected: ${} from each player (total: ${})", ante_amount, total_ante);
                            }
                        }
                    }
                }
            }
            
            // Check for Showdown phase FIRST - ALL players should see this
            if state.contains("Showdown") {
                handle_showdown(conn);
                break;
            }
            
            // Check phase FIRST before checking turn
            if state.contains("Draw") && state.contains("Your turn: true") {
                display_table_layout(conn);
                if !handle_draw_turn(conn) {
                    continue; // Loop back to check game state
                }
            } else if state.contains("BettingRound") && state.contains("Your turn: true") {
                display_table_layout(conn);
                if !handle_betting_turn(conn) {
                    continue; // Loop back to check game state
                }
            } else {
                // Not my turn, waiting - but check for Showdown phase first
                if state.contains("Showdown") {
                    handle_showdown(conn);
                    break;
                } else if state.contains("WaitingForPlayers") {
                    // Game was reset - check if we can get showdown results
                    conn.send_command("SHOWDOWN");
                    let showdown_result = conn.read_response();
                    if showdown_result.starts_with("OK SHOWDOWN") {
                        // Format showdown nicely
                        format_showdown_result(&showdown_result);
                    }
                    break;
                } else {
                    // Not my turn, waiting - use smart waiting display
                    display_waiting_status(conn, &mut previous_state);
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
            
            // Check if game is complete
            if state.contains("Showdown") || state.contains("BetweenRounds") {
                break;
            }
        }
        
        // Ask if play another hand
        let again = get_input("\nPlay another hand? [y/n]: ", |s| {
            let s = s.trim().to_lowercase();
            if s == "y" || s == "yes" {
                Some(true)
            } else if s == "n" || s == "no" {
                Some(false)
            } else {
                None
            }
        }).unwrap();
        
        if !again {
            // Leave table before exiting
            conn.send_command("LEAVE_TABLE");
            let leave_response = conn.read_response();
            if leave_response.starts_with("OK LEFT_TABLE") {
                println!("Left the table.");
            }
            return; // Exit play_game function
        }
    }
}

fn choose_variant(conn: &mut ClientConnection) {
    println!("\nSelect game variant:");
    println!("  [1] Five-Card Draw");
    println!("  [2] Texas Hold'em");
    let choice = get_input("Choice [1/2]: ", |s| {
        let t = s.trim().to_string();
        if t == "1" || t == "2" {
            Some(t)
        } else {
            None
        }
    })
    .unwrap_or_else(|| "2".to_string());
    let cmd = if choice == "1" {
        "SET_VARIANT DRAW"
    } else {
        "SET_VARIANT HOLDEM"
    };
    conn.send_command(cmd);
    println!("{}", conn.read_response());
}

/// Get player count from table
fn get_player_count(conn: &mut ClientConnection) -> usize {
    conn.send_command("GET_TABLE_INFO");
    let table_info = conn.read_response();
    if table_info.starts_with("OK TABLE_INFO") {
        table_info.matches("Seat").count()
    } else {
        0
    }
}

/// Wait for game to start or start it
/// Returns true if game started, false if player left
fn wait_for_game_start(conn: &mut ClientConnection) -> bool {
    let mut previous_player_count: Option<usize> = None;
    let mut first_display = true;
    
    loop {
        conn.send_command("GET_GAME_STATE");
        let state = conn.read_response();
        
        if state.contains("WaitingForPlayers") {
            // Check for updates periodically
            let current_count = get_player_count(conn);
            
            // Show updates when player count changes
            if let Some(prev_count) = previous_player_count {
                if current_count != prev_count {
                    if current_count > prev_count {
                        println!("\n✓ Player joined! {} players at table now.", current_count);
                    } else if current_count < prev_count {
                        println!("\n⚠ Player left. {} players at table now.", current_count);
                    }
                    first_display = true; // Force full redisplay
                }
            }
            
            if first_display {
                // Full display
                println!("\n=== Waiting for Players ===");
                println!("Players at table: {}", current_count);
                if current_count >= 2 && current_count <= 5 {
                    println!("✓ Ready to start! (Need 2-5 players)");
                } else if current_count < 2 {
                    println!("⏳ Waiting for more players... (Need {} more)", 2 - current_count);
                } else {
                    println!("⚠ Too many players! (Max 5)");
                }
                first_display = false;
            }
            
            previous_player_count = Some(current_count);
            
            // Show menu options
            println!("\nOptions:");
            println!("  [s] Start game");
            println!("  [l] Leave table");
            println!("  [t] Show table");
            println!("  [Enter] Refresh status");
            
            // Use a simple input that allows just pressing enter to refresh
            print!("\nEnter choice (or press Enter to refresh): ");
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            let choice = input.trim().to_lowercase();
            
            if choice.is_empty() {
                // Just refresh - clear and continue loop
                println!("\nRefreshing...");
                std::thread::sleep(std::time::Duration::from_millis(500));
                first_display = true; // Force refresh
                continue;
            }
            
            match choice.as_str() {
                "s" | "start" => {
                    conn.send_command("START_GAME");
                    let response = conn.read_response();
                    println!("{}", response);
                if response.starts_with("OK GAME_STARTED") {
                    // Ante message will be shown in the game loop for all players
                    // No need to show it here to avoid duplicate messages
                    return true;
                } else if response.starts_with("OK GAME_IN_PROGRESS") {
                        println!("Joining ongoing game...");
                        // Show ante info when joining ongoing game
                        conn.send_command("GET_GAME_STATE");
                        let state = conn.read_response();
                        if let Some(ante_part) = state.split("Ante: ").nth(1) {
                            if let Some(ante_str) = ante_part.split(' ').next() {
                                if let Ok(ante_amount) = ante_str.parse::<i32>() {
                                    if ante_amount > 0 {
                                        conn.send_command("GET_TABLE_INFO");
                                        let table_info = conn.read_response();
                                        let player_count = if table_info.contains("Seat") {
                                            table_info.matches("Seat").count()
                                        } else {
                                            0
                                        };
                                        let total_ante = ante_amount * player_count as i32;
                                        println!("💰 Ante collected: ${} from each player (total: ${})", ante_amount, total_ante);
                                    }
                                }
                            }
                        }
                        return true;
                    } else if !response.starts_with("OK") {
                        println!("Could not start game. {}", response);
                        // Reset to check again
                        previous_player_count = None;
                        first_display = true;
                        continue;
                    }
                }
                "l" | "leave" => {
                    conn.send_command("LEAVE_TABLE");
                    let response = conn.read_response();
                    println!("{}", response);
                    if response.starts_with("OK LEFT_TABLE") {
                        return false; // Signal that player left
                    }
                }
                "t" | "table" => {
                    display_table_layout(conn);
                    // After showing table, continue loop to refresh count
                    first_display = true;
                    continue;
                }
                _ => {
                    println!("Invalid choice. Please try again.");
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    first_display = true;
                    continue;
                }
            }
        } else {
            // Game already started or in progress
            println!("Game is in progress. Joining...");
            return true;
        }
    }
}

/// Handle betting turn
/// Returns false if betting round completed, true otherwise
fn handle_betting_turn(conn: &mut ClientConnection) -> bool {
    conn.send_command("GET_HAND");
    let hand_response = conn.read_response();
    let hand = if hand_response.starts_with("OK HAND") {
        hand_response.strip_prefix("OK HAND ").unwrap_or("")
    } else {
        ""
    };
    
    println!("\n=== Your Turn ===");
    conn.send_command("GET_GAME_STATE");
    let state = conn.read_response();
    display_game_state(&state);
    if !hand.is_empty() {
        println!("Your hand: {}", hand);
    }
    
    // Determine available actions based on bet_to_call
    let bet_to_call = if let Some(btc_str) = state.split("Bet to call: ").nth(1) {
        btc_str.split(' ').next().unwrap_or("0").parse::<i32>().unwrap_or(0)
    } else {
        0
    };
    
    let action_prompt = if bet_to_call == 0 {
        "Enter action (bet <amt> / check / fold): "
    } else {
        "Enter action (call / raise <amt> / fold): "
    };
    
    let action = get_input(action_prompt, |s| {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }).unwrap_or_else(|| {
        println!("Please enter a valid action.");
        "".to_string()
    });
    
    if action.is_empty() {
        return true;
    }
    
    // Parse and validate action before sending
    let command = if action.to_lowercase().starts_with("bet") {
        let amount_str = action.split_whitespace().nth(1);
        if let Some(amt_str) = amount_str {
            if let Ok(amt) = amt_str.parse::<i32>() {
                if amt > 0 {
                    format!("BET {}", amt)
                } else {
                    println!("ERROR Bet amount must be positive");
                    return true;
                }
            } else {
                println!("ERROR Invalid bet amount: {}", amt_str);
                return true;
            }
        } else {
            println!("ERROR Usage: bet <amount>");
            return true;
        }
    } else if action.to_lowercase().starts_with("call") {
        "CALL".to_string()
    } else if action.to_lowercase().starts_with("raise") {
        let amount_str = action.split_whitespace().nth(1);
        if let Some(amt_str) = amount_str {
            if let Ok(amt) = amt_str.parse::<i32>() {
                if amt > bet_to_call {
                    format!("RAISE {}", amt)
                } else {
                    println!("ERROR Raise amount ({}) must be greater than current bet to call ({})", amt, bet_to_call);
                    return true;
                }
            } else {
                println!("ERROR Invalid raise amount: {}", amt_str);
                return true;
            }
        } else {
            println!("ERROR Usage: raise <amount>");
            return true;
        }
    } else if action.to_lowercase().starts_with("fold") {
        "FOLD".to_string()
    } else if action.to_lowercase().starts_with("check") {
        "CHECK".to_string()
    } else {
        println!("ERROR Invalid action. Valid actions: bet <amt>, call, raise <amt>, fold, check");
        return true;
    };
    
    // Re-check game state right before sending to avoid race conditions
    conn.send_command("GET_GAME_STATE");
    let verify_state = conn.read_response();
    if !verify_state.contains("BettingRound") || !verify_state.contains("Your turn: true") {
        println!("ERROR Game state changed. It's no longer your turn.");
        return true;
    }
    
    conn.send_command(&command);
    let response = conn.read_response();
    
    // Check if action was successful
    if response.starts_with("ERROR") {
        println!("{}", response);
        // Don't break - let player try again
        return true;
    } else {
        println!("{}", response);
    }
    
    // If betting round completed, continue to check for phase transition
    if response.contains("Betting round complete") {
        return false; // Signal to continue loop
    }
    
    true
}

/// Handle draw turn
/// Returns false if all draws complete, true otherwise
fn handle_draw_turn(conn: &mut ClientConnection) -> bool {
    conn.send_command("GET_HAND");
    let hand_response = conn.read_response();
    let hand = if hand_response.starts_with("OK HAND") {
        hand_response.strip_prefix("OK HAND ").unwrap_or("")
    } else {
        ""
    };
    
    println!("\n=== Draw Phase ===");
    println!("Your hand: {}", hand);
    let draw_input = get_input("Enter card positions to replace (0-4, space-separated, or 'none'): ", 
        |s| Some(s.trim().to_string())).unwrap();
    
    let command = if draw_input.to_lowercase() == "none" || draw_input.trim().is_empty() {
        "DRAW none".to_string()
    } else {
        format!("DRAW {}", draw_input)
    };
    
    conn.send_command(&command);
    let response = conn.read_response();
    println!("{}", response);
    
    // If all draws complete, continue to check for next phase
    if response.contains("All draws complete") {
        return false; // Signal to continue loop
    }
    
    true
}

/// Handle showdown phase
/// Format showdown result into multiple readable lines
fn format_showdown_result(showdown_result: &str) {
    if !showdown_result.starts_with("OK SHOWDOWN") {
        println!("{}", showdown_result);
        return;
    }
    
    let content = showdown_result.strip_prefix("OK SHOWDOWN ").unwrap_or("");
    
    println!("\n=== SHOWDOWN RESULTS ===");
    
    // Parse "All hands: ... | Winners: ..."
    if let Some(hands_part) = content.split(" | Winners: ").next() {
        let hands_str = hands_part.strip_prefix("All hands: ").unwrap_or("");
        let hands: Vec<&str> = hands_str.split(" | ").collect();
        
        println!("\nAll Hands:");
        for hand_info in &hands {
            // Format: "Player: cards - HandRank"
            if let Some((player_part, rest)) = hand_info.split_once(": ") {
                if let Some((cards, rank)) = rest.rsplit_once(" - ") {
                    println!("  • {}: {} ({})", player_part, cards, rank);
                } else {
                    println!("  • {}", hand_info);
                }
            } else {
                println!("  • {}", hand_info);
            }
        }
    }
    
    if let Some(winners_part) = content.split(" | Winners: ").nth(1) {
        let winners = winners_part.split(" | ").next().unwrap_or("");
        println!("\n🏆 WINNER(S): {}", winners);
    }
    
    if content.contains("Pot distributed") {
        println!("✓ Pot distributed. Ready for next hand!");
    }
    
    println!();
}

fn handle_showdown(conn: &mut ClientConnection) {
    // Get final hands
    conn.send_command("GET_HAND");
    let hand = conn.read_response();
    let my_hand = if hand.starts_with("OK HAND") {
        hand.strip_prefix("OK HAND ").unwrap_or("")
    } else {
        ""
    };
    
    // Trigger showdown processing (any player can trigger, but it only processes once)
    conn.send_command("SHOWDOWN");
    let showdown_result = conn.read_response();
    
    // Format showdown results nicely
    if !my_hand.is_empty() {
        println!("\nYour Final Hand: {}", my_hand);
    }
    
    format_showdown_result(&showdown_result);
    
    // Wait a bit for server to process
    std::thread::sleep(std::time::Duration::from_millis(500));
}

/// Display game state in a clean, readable format
fn display_game_state(state_str: &str) {
    if !state_str.starts_with("OK GAME_STATE") {
        println!("{}", state_str);
        return;
    }
    
    let content = state_str.strip_prefix("OK GAME_STATE ").unwrap_or("");
    
    // Parse the pipe-separated values
    let mut variant = "";
    let mut street = "";
    let mut board = "";
    let mut blinds = "";
    let mut phase = "";
    let mut pot = "";
    let mut bet_to_call = "";
    let mut your_turn = "";
    let mut current_actor = "";
    let mut next_info = "";
    let mut until_turn = "";
    
    for part in content.split(" | ") {
        if part.starts_with("Variant: ") {
            variant = part.strip_prefix("Variant: ").unwrap_or("");
        } else if part.starts_with("Street: ") {
            street = part.strip_prefix("Street: ").unwrap_or("");
        } else if part.starts_with("Board: ") {
            board = part.strip_prefix("Board: ").unwrap_or("");
        } else if part.starts_with("Blinds: ") {
            blinds = part.strip_prefix("Blinds: ").unwrap_or("");
        } else if part.starts_with("Phase: ") {
            phase = part.strip_prefix("Phase: ").unwrap_or("");
        } else if part.starts_with("Pot: ") {
            pot = part.strip_prefix("Pot: ").unwrap_or("");
        } else if part.starts_with("Bet to call: ") {
            bet_to_call = part.strip_prefix("Bet to call: ").unwrap_or("");
        } else if part.starts_with("Your turn: ") {
            your_turn = part.strip_prefix("Your turn: ").unwrap_or("");
        } else if part.starts_with("Current actor: ") {
            current_actor = part.strip_prefix("Current actor: ").unwrap_or("");
        } else if part.starts_with("Next: ") {
            next_info = part.strip_prefix("Next: ").unwrap_or("");
        } else if part.starts_with("Until your turn: ") {
            until_turn = part.strip_prefix("Until your turn: ").unwrap_or("");
        }
    }
    
    // Format phase name nicely
    let phase_display = match phase {
        "BettingRound" => "Betting Round",
        "Draw" => "Draw Phase",
        "Showdown" => "Showdown",
        "WaitingForPlayers" => "Waiting for Players",
        _ => phase,
    };
    
    println!("\n  Game Status");
    if !variant.is_empty() {
        println!("  Variant: {}", variant);
    }
    if !street.is_empty() && street != "N/A" {
        println!("  Street: {}", street);
    }
    if !board.is_empty() && board != "N/A" {
        println!("  Board: {}", board);
    }
    if !blinds.is_empty() && blinds != "N/A" {
        println!("  Blinds: {}", blinds);
    }
    println!("  Phase: {}", phase_display);
    println!("  Pot: ${}", pot);
    if bet_to_call != "0" {
        println!("  Bet to call: ${}", bet_to_call);
    }
    
    if your_turn == "true" {
        println!("  It's your turn!");
    } else {
        println!("  ⏳ Current actor: {}", current_actor);
        if next_info != "N/A" && next_info != "Unknown" {
            if next_info == "You" {
                println!("  👉 You're next!");
            } else if until_turn != "0" {
                println!("  👉 Next: {} ({} player(s) until your turn)", next_info, until_turn);
            } else {
                println!("  👉 Next: {}", next_info);
            }
        }
    }
    println!();
}

/// Display table layout with seats, dealer, current actor, etc.
fn display_table_layout(conn: &mut ClientConnection) {
    conn.send_command("GET_TABLE_INFO");
    let response = conn.read_response();
    
    if response.starts_with("OK TABLE_INFO") {
        let info = response.strip_prefix("OK TABLE_INFO ").unwrap_or("");
        
        // Parse and format the table info nicely
        let parts: Vec<&str> = info.split(" | ").collect();
        let mut seats = Vec::new();
        let mut pot = "";
        let mut bet_to_call = "";
        
        for part in &parts {
            if part.starts_with("Seat") {
                seats.push(*part);
            } else if part.starts_with("Pot:") {
                pot = part;
            } else if part.starts_with("Bet to call:") {
                bet_to_call = part;
            }
        }
        
        if !seats.is_empty() {
            println!("\n┌─ Table ──────────────────────────────┐");
            for seat in &seats {
                println!("│  {}", seat);
            }
            if !pot.is_empty() {
                println!("│  {}", pot);
            }
            if !bet_to_call.is_empty() {
                println!("│  {}", bet_to_call);
            }
            println!("└──────────────────────────────────────┘\n");
        }
    }
}

/// Display waiting status with current actor and table info
/// Only prints once per state change (tracks previous state)
fn display_waiting_status(conn: &mut ClientConnection, previous_state: &mut Option<String>) {
    conn.send_command("GET_GAME_STATE");
    let state = conn.read_response();
    
    // Only display if state changed
    if let Some(prev) = previous_state {
        if *prev == state {
            return; // State hasn't changed, don't print again
        }
    }
    *previous_state = Some(state.clone());
    
    // Parse state for current actor and next info
    let current_actor = if let Some(start) = state.find("Current actor: ") {
        let rest = &state[start + 15..];
        if let Some(end) = rest.find(" |") {
            rest[..end].to_string()
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    };
    
    // Display single informative message
    println!("⏳ Waiting... {} is acting.", current_actor);
    
    // Display table layout
    display_table_layout(conn);
}
