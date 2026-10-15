//! Authentication command handlers
//! Handles LOGIN and SIGNUP commands

use crate::state::ServerState;
use common::accounts::player::Player;
use poker::persistence::storage::{
    load_player_by_username, load_player_by_username_and_pin, save_player,
};

/// Validate PIN token: digits only, length >= 4, parse i32
fn parse_pin_token(tok: &str) -> Result<i32, String> {
    if tok.len() < 4 {
        return Err("PIN must be at least 4 digits".to_string());
    }
    if !tok.chars().all(|c| c.is_ascii_digit()) {
        return Err("Invalid PIN format".to_string());
    }
    tok.parse::<i32>()
        .map_err(|_| "Invalid PIN format".to_string())
}

/// Validate username token: non-empty, no whitespace, case-insensitive (stored/queried lowercase)
fn parse_username_token(tok: &str) -> Result<String, String> {
    let u = tok.trim();
    if u.is_empty() || u.chars().any(|c| c.is_whitespace()) {
        return Err("Invalid username, ensure it's single word (no whitespaces)".to_string());
    }
    Ok(u.to_lowercase())
}

/// Handle LOGIN command
pub fn handle_login(
    parts: &[&str],
    player: &mut Option<Player>,
    server_state: &ServerState,
) -> String {
    if parts.len() < 3 {
        return "ERROR Usage: LOGIN <username> <pin>".to_string();
    }

    let username = match parse_username_token(parts[1]) {
        Ok(u) => u,
        Err(e) => return format!("ERROR {}", e),
    };

    let pin = match parse_pin_token(parts[2]) {
        Ok(p) => p,
        Err(e) => return format!("ERROR {}", e),
    };

    match load_player_by_username_and_pin(&username, pin) {
        Some(p) => {
            *player = Some(p.clone());
            let mut connected = server_state.connected_players.lock().unwrap();

            //ensure no duplicated logins:
            if connected.contains_key(&p.id) {
                return "ERROR Player already logged in".to_string();
            }

            connected.insert(p.id, p.clone());
            format!("OK LOGIN Welcome, {}! Balance: {}", p.name, p.balance)
        }
        None => "ERROR Invalid username or PIN".to_string(),
    }
}

/// Handle SIGNUP command
pub fn handle_signup(
    parts: &[&str],
    player: &mut Option<Player>,
    server_state: &ServerState,
) -> String {
    if parts.len() < 4 {
        return "ERROR Usage: SIGNUP <username> <pin> <balance>".to_string();
    }

    let username = parts[1].trim();
    if username.is_empty() || username.chars().any(|c| c.is_whitespace()) {
        return "ERROR Username must be one word (no spaces)".to_string();
    }

    let pin = match parse_pin_token(parts[2]) {
        Ok(p) => p,
        Err(e) => return format!("ERROR {}", e),
    };

    let balance = match parts[3].parse::<i32>() {
        Ok(b) if b > 0 => b,
        Ok(_) => return "ERROR Balance must be positive".to_string(),
        Err(_) => return "ERROR Invalid balance".to_string(),
    };

    // Reject duplicate username
    if load_player_by_username(username).is_some() {
        return "ERROR Username already exists".to_string();
    }

    let new_player = Player::new(username, pin, balance);

    match save_player(&new_player) {
        Ok(_) => {
            *player = Some(new_player.clone());
            let mut connected = server_state.connected_players.lock().unwrap();
            connected.insert(new_player.id, new_player.clone());
            format!(
                "OK SIGNUP Account created! Welcome, {}! Balance: {}",
                new_player.name, new_player.balance
            )
        }
        Err(e) => format!("ERROR Failed to save account: {}", e),
    }
}
