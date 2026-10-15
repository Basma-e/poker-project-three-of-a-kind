//! TCP Server for Poker Game - Phase 1
//! Listens for client connections and manages game state

mod commands;
mod network;
mod state;

use crate::commands::process_command;
use crate::network::send_response;
use crate::state::ServerState;
use common::accounts::player::Player;
use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

fn main() {
    let server_state = Arc::new(ServerState::new());
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind to port 8080");

    println!("Poker Server listening on 127.0.0.1:8080");
    println!("Waiting for clients to connect...\n");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&server_state);
                thread::spawn(move || {
                    handle_client(stream, state);
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream, server_state: Arc<ServerState>) {
    let mut player: Option<Player> = None;
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    // Send welcome message
    send_response(&mut stream, "WELCOME Poker Server - Phase 1");

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // Client disconnected
            Ok(_) => {
                let command = line.trim();
                let response = process_command(command, &mut player, &server_state);

                if response.starts_with("OK QUIT") {
                    send_response(&mut stream, &response);
                    break;
                }

                send_response(&mut stream, &response);
            }
            Err(e) => {
                eprintln!("Error reading from client: {}", e);
                break;
            }
        }
    }

    // Cleanup: remove player from connected list and table if game is waiting
    if let Some(p) = player.as_ref() {
        let mut connected = server_state.connected_players.lock().unwrap();
        connected.remove(&p.id);
        drop(connected);

        // If game is waiting for players, remove from table too
        let table_opt = server_state.table.lock().unwrap();
        if let Some(ref table) = table_opt.as_ref() {
            let mut table_guard = table.lock().unwrap();
            if table_guard.state.phase == poker::table::state::GamePhase::WaitingForPlayers {
                let initial_len = table_guard.players.len();
                table_guard.players.retain(|tp| tp.id != p.id);
                if table_guard.players.len() < initial_len {
                    // Player was removed, save remaining players
                    for player_at_table in &table_guard.players {
                        let _ = poker::persistence::storage::save_player(player_at_table);
                    }
                }
            }
        }
    }
}
