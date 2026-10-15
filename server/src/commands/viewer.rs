//! Public/viewer-only game info handlers.
//! These commands expose table/game visibility without allowing gameplay actions.

use crate::state::ServerState;
use common::accounts::player::Player;
use poker::table::state::GamePhase;

pub fn handle_watch_game(player: &Option<Player>, _server_state: &ServerState) -> String {
    if player.is_none() {
        return "ERROR Not logged in".to_string();
    }
    "OK WATCHING Connected to table 1 as viewer".to_string()
}

pub fn handle_stop_watching(player: &Option<Player>, _server_state: &ServerState) -> String {
    if player.is_none() {
        return "ERROR Not logged in".to_string();
    }
    "OK STOPPED_WATCHING".to_string()
}

pub fn handle_get_public_table_info(player: &Option<Player>, server_state: &ServerState) -> String {
    if player.is_none() {
        return "ERROR Not logged in".to_string();
    }

    let table_opt = server_state.table.lock().unwrap();
    if let Some(ref table) = table_opt.as_ref() {
        let table_guard = table.lock().unwrap();
        let mut seat_info = Vec::new();
        for (seat, player_at_seat) in table_guard.players.iter().enumerate() {
            let status = if seat < table_guard.state.player_bet_states.len()
                && table_guard.state.player_bet_states[seat].folded
            {
                "Folded"
            } else if table_guard.state.phase == GamePhase::BettingRound
                || table_guard.state.phase == GamePhase::Draw
            {
                if seat == table_guard.state.current_actor_seat {
                    "Acting"
                } else {
                    "Active"
                }
            } else {
                "Active"
            };

            let dealer_tag = if seat == table_guard.state.dealer_seat {
                " [Dealer]"
            } else {
                ""
            };
            seat_info.push(format!(
                "Seat {}: {}{} ({})",
                seat, player_at_seat.name, dealer_tag, status
            ));
        }

        format!(
            "OK PUBLIC_TABLE_INFO Table ID: 1 | Variant: {} | Pot: {} | Dealer: seat {} | Players: {}",
            table_guard.variant.display_name(),
            table_guard.state.pot,
            table_guard.state.dealer_seat,
            seat_info.join(" | ")
        )
    } else {
        "ERROR No active table".to_string()
    }
}

pub fn handle_get_public_game_state(player: &Option<Player>, server_state: &ServerState) -> String {
    if player.is_none() {
        return "ERROR Not logged in".to_string();
    }
    let table_opt = server_state.table.lock().unwrap();
    let engine_opt = server_state.game_engine.lock().unwrap();
    if let (Some(ref table), Some(ref engine)) = (table_opt.as_ref(), engine_opt.as_ref()) {
        let table_guard = table.lock().unwrap();
        let engine_guard = engine.lock().unwrap();
        let street = engine_guard
            .holdem_street()
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "N/A".to_string());
        let board = engine_guard
            .holdem_community_cards()
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        format!(
            "OK PUBLIC_GAME_STATE Table ID: 1 | Variant: {} | Phase: {:?} | Round: {} | Pot: {} | Dealer: seat {} | Current actor: seat {} | Public board: {}",
            table_guard.variant.display_name(),
            table_guard.state.phase,
            street,
            table_guard.state.pot,
            table_guard.state.dealer_seat,
            table_guard.state.current_actor_seat,
            if board.is_empty() {
                "N/A".to_string()
            } else {
                board
            }
        )
    } else {
        "ERROR No active game".to_string()
    }
}

pub fn handle_get_public_results(player: &Option<Player>, server_state: &ServerState) -> String {
    if player.is_none() {
        return "ERROR Not logged in".to_string();
    }
    let last_showdown = server_state.last_showdown.lock().unwrap();
    if let Some(ref result) = *last_showdown {
        result.clone()
    } else {
        "OK SHOWDOWN No hand results yet".to_string()
    }
}
