//! Command router module
//! Routes incoming commands to appropriate handlers

mod account;
mod auth;
mod betting;
mod draw;
mod game;
mod table;
mod viewer;

use crate::state::ServerState;
use common::accounts::player::Player;

const MAX_COMMAND_LEN: usize = 1024;

fn ensure_exact_args(parts: &[&str], expected: usize, usage: &str) -> Result<(), String> {
    if parts.len() == expected {
        Ok(())
    } else {
        Err(format!("ERROR Usage: {}", usage))
    }
}

/// Process a command and return a response
pub fn process_command(
    command: &str,
    player: &mut Option<Player>,
    server_state: &ServerState,
) -> String {
    if command.len() > MAX_COMMAND_LEN {
        return format!("ERROR Command too long (max {} bytes)", MAX_COMMAND_LEN);
    }
    if command.chars().any(|c| c.is_control() && !c.is_whitespace()) {
        return "ERROR Command contains invalid control characters".to_string();
    }

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return "ERROR Empty command".to_string();
    }

    match parts[0] {
        "LOGIN" => match ensure_exact_args(&parts, 3, "LOGIN <username> <pin>") {
            Ok(_) => auth::handle_login(&parts, player, server_state),
            Err(e) => e,
        },
        "SIGNUP" => match ensure_exact_args(&parts, 4, "SIGNUP <username> <pin> <balance>") {
            Ok(_) => auth::handle_signup(&parts, player, server_state),
            Err(e) => e,
        },
        "VIEW_STATS" => match ensure_exact_args(&parts, 1, "VIEW_STATS") {
            Ok(_) => account::handle_view_stats(player, server_state),
            Err(e) => e,
        },
        "VIEW_HISTORY" => match ensure_exact_args(&parts, 1, "VIEW_HISTORY") {
            Ok(_) => account::handle_view_history_lines(player, server_state),
            Err(e) => e,
        },
        "DEPOSIT" => match ensure_exact_args(&parts, 2, "DEPOSIT <amount>") {
            Ok(_) => account::handle_deposit(&parts, player, server_state),
            Err(e) => e,
        },
        "WITHDRAW" => match ensure_exact_args(&parts, 2, "WITHDRAW <amount>") {
            Ok(_) => account::handle_withdraw(&parts, player, server_state),
            Err(e) => e,
        },
        "BALANCE" => match ensure_exact_args(&parts, 1, "BALANCE") {
            Ok(_) => account::handle_balance(player, server_state),
            Err(e) => e,
        },
        "JOIN_TABLE" => match ensure_exact_args(&parts, 1, "JOIN_TABLE") {
            Ok(_) => table::handle_join_table(player, server_state),
            Err(e) => e,
        },
        "LEAVE_TABLE" => match ensure_exact_args(&parts, 1, "LEAVE_TABLE") {
            Ok(_) => table::handle_leave_table(player, server_state),
            Err(e) => e,
        },
        "START_GAME" => match ensure_exact_args(&parts, 1, "START_GAME") {
            Ok(_) => table::handle_start_game(player, server_state),
            Err(e) => e,
        },
        "SET_VARIANT" => match ensure_exact_args(&parts, 2, "SET_VARIANT <DRAW|HOLDEM>") {
            Ok(_) => table::handle_set_variant(&parts, player, server_state),
            Err(e) => e,
        },
        "GET_TABLE_INFO" => match ensure_exact_args(&parts, 1, "GET_TABLE_INFO") {
            Ok(_) => table::handle_get_table_info(player, server_state),
            Err(e) => e,
        },
        "GET_GAME_STATE" => match ensure_exact_args(&parts, 1, "GET_GAME_STATE") {
            Ok(_) => game::handle_get_game_state(player, server_state),
            Err(e) => e,
        },
        "GET_HAND" => match ensure_exact_args(&parts, 1, "GET_HAND") {
            Ok(_) => game::handle_get_hand(player, server_state),
            Err(e) => e,
        },
        "SHOWDOWN" => match ensure_exact_args(&parts, 1, "SHOWDOWN") {
            Ok(_) => game::handle_showdown(player, server_state),
            Err(e) => e,
        },
        "WATCH_GAME" => match ensure_exact_args(&parts, 1, "WATCH_GAME") {
            Ok(_) => viewer::handle_watch_game(player, server_state),
            Err(e) => e,
        },
        "STOP_WATCHING" => match ensure_exact_args(&parts, 1, "STOP_WATCHING") {
            Ok(_) => viewer::handle_stop_watching(player, server_state),
            Err(e) => e,
        },
        "GET_PUBLIC_TABLE_INFO" => match ensure_exact_args(&parts, 1, "GET_PUBLIC_TABLE_INFO") {
            Ok(_) => viewer::handle_get_public_table_info(player, server_state),
            Err(e) => e,
        },
        "GET_PUBLIC_GAME_STATE" => match ensure_exact_args(&parts, 1, "GET_PUBLIC_GAME_STATE") {
            Ok(_) => viewer::handle_get_public_game_state(player, server_state),
            Err(e) => e,
        },
        "GET_PUBLIC_RESULTS" => match ensure_exact_args(&parts, 1, "GET_PUBLIC_RESULTS") {
            Ok(_) => viewer::handle_get_public_results(player, server_state),
            Err(e) => e,
        },
        "BET" => match ensure_exact_args(&parts, 2, "BET <amount>") {
            Ok(_) => betting::handle_betting_action(command, player, server_state),
            Err(e) => e,
        },
        "CALL" => match ensure_exact_args(&parts, 1, "CALL") {
            Ok(_) => betting::handle_betting_action(command, player, server_state),
            Err(e) => e,
        },
        "RAISE" => match ensure_exact_args(&parts, 2, "RAISE <amount>") {
            Ok(_) => betting::handle_betting_action(command, player, server_state),
            Err(e) => e,
        },
        "FOLD" => match ensure_exact_args(&parts, 1, "FOLD") {
            Ok(_) => betting::handle_betting_action(command, player, server_state),
            Err(e) => e,
        },
        "CHECK" => match ensure_exact_args(&parts, 1, "CHECK") {
            Ok(_) => betting::handle_betting_action(command, player, server_state),
            Err(e) => e,
        },
        "DRAW" => {
            if parts.len() < 2 {
                "ERROR Usage: DRAW <positions> or DRAW none".to_string()
            } else {
                draw::handle_draw_action(command, player, server_state)
            }
        }
        "QUIT" => match ensure_exact_args(&parts, 1, "QUIT") {
            Ok(_) => "OK QUIT Goodbye!".to_string(),
            Err(e) => e,
        },
        _ => format!("ERROR Unknown command: {}", parts[0]),
    }
}
