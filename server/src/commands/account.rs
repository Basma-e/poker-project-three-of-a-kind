//! Account management command handlers
//! Handles VIEW_STATS, VIEW_HISTORY, DEPOSIT, WITHDRAW, and BALANCE commands

use common::accounts::history::HistoryEvent;
use common::accounts::player::Player;
use poker::persistence::storage::save_player;

use crate::state::ServerState;
use chrono::Utc;

fn fresh_player(player: &Option<Player>, server_state: &ServerState) -> Option<Player> {
    let p = player.as_ref()?;
    let connected = server_state.connected_players.lock().unwrap();
    connected.get(&p.id).cloned().or_else(|| Some(p.clone()))
}

/// Handle VIEW_STATS command
pub fn handle_view_stats(player: &Option<Player>, server_state: &ServerState) -> String {
    if let Some(p) = fresh_player(player, server_state) {
        format!(
            "OK STATS Rounds: {} | Folds: {} | Pots Won: {}",
            p.stats.rounds_played, p.stats.folds, p.stats.pots_won
        )
    } else {
        "ERROR Not logged in".to_string()
    }
}

/// Handle VIEW_HISTORY command
pub fn handle_view_history_lines(player: &Option<Player>, server_state: &ServerState) -> String {
    if let Some(p) = fresh_player(player, server_state) {
        if p.history.is_empty() {
            return "OK HISTORY No history available".to_string();
        }

        let history_str = p
            .history
            .iter()
            .map(|e| match e {
                HistoryEvent::Deposit { amount, timestamp } => {
                    format!("[{}] Deposit: +{}", timestamp, amount)
                }
                HistoryEvent::Withdraw { amount, timestamp } => {
                    format!("[{}] Withdraw: -{}", timestamp, amount)
                }
                HistoryEvent::HandResult {
                    pot_won,
                    timestamp,
                    game_type,
                } => {
                    let gt = game_type
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string());
                    format!("[{}] Hand won: +{} ({})", timestamp, pot_won, gt)
                }
            })
            .collect::<Vec<_>>()
            .join(" || ");

        format!("OK HISTORY {}", history_str)
    } else {
        "ERROR Not logged in".to_string()
    }
}

/// Handle BALANCE command
pub fn handle_balance(player: &Option<Player>, server_state: &ServerState) -> String {
    if let Some(p) = fresh_player(player, server_state) {
        format!("OK BALANCE {}", p.balance)
    } else {
        "ERROR Not logged in".to_string()
    }
}

/// Handle DEPOSIT command
pub fn handle_deposit(
    parts: &[&str],
    player: &mut Option<Player>,
    server_state: &ServerState,
) -> String {
    if let Some(p) = player.as_mut() {
        if parts.len() < 2 {
            return "ERROR Usage: DEPOSIT <amount>".to_string();
        }
        match parts[1].parse::<i32>() {
            Ok(amount) if amount > 0 => {
                p.balance += amount;
                p.history.push(HistoryEvent::Deposit {
                    amount,
                    timestamp: Utc::now(),
                });
                match save_player(p) {
                    Ok(_) => {
                        let mut connected = server_state.connected_players.lock().unwrap();
                        connected.insert(p.id, p.clone());
                        format!(
                            "OK DEPOSIT Deposited {}. New balance: {}",
                            amount, p.balance
                        )
                    }
                    Err(e) => format!("ERROR Failed to save: {}", e),
                }
            }
            Ok(_) => "ERROR Amount must be positive".to_string(),
            Err(_) => "ERROR Invalid amount".to_string(),
        }
    } else {
        "ERROR Not logged in".to_string()
    }
}

/// Handle WITHDRAW command
pub fn handle_withdraw(
    parts: &[&str],
    player: &mut Option<Player>,
    server_state: &ServerState,
) -> String {
    if let Some(p) = player.as_mut() {
        if parts.len() < 2 {
            return "ERROR Usage: WITHDRAW <amount>".to_string();
        }

        match parts[1].parse::<i32>() {
            Ok(amount) if amount > 0 => {
                if p.balance < amount {
                    return format!(
                        "ERROR Insufficient funds. Balance: {}, requested: {}",
                        p.balance, amount
                    );
                }

                p.balance -= amount;
                p.history.push(HistoryEvent::Withdraw {
                    amount,
                    timestamp: Utc::now(),
                });

                match save_player(p) {
                    Ok(_) => {
                        let mut connected = server_state.connected_players.lock().unwrap();
                        connected.insert(p.id, p.clone());
                        format!(
                            "OK WITHDRAW Withdrew {}. New balance: {}",
                            amount, p.balance
                        )
                    }
                    Err(e) => format!("ERROR Failed to save: {}", e),
                }
            }
            Ok(_) => "ERROR Amount must be positive".to_string(),
            Err(_) => "ERROR Invalid amount".to_string(),
        }
    } else {
        "ERROR Not logged in".to_string()
    }
}
