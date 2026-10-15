//! Server state management
//! Shared game state across all client connections

use common::accounts::player::{Player, PlayerId};
use poker::game::GameEngine;
use poker::table::table::Table;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Shared game state for the poker server
pub struct ServerState {
    /// Active table (for Phase 1, single table)
    pub table: Arc<Mutex<Option<Arc<Mutex<Table>>>>>,
    /// Game engine for managing hands and gameplay
    pub game_engine: Arc<Mutex<Option<Arc<Mutex<GameEngine>>>>>,
    /// Connected players: player_id -> player_data
    pub connected_players: Arc<Mutex<HashMap<PlayerId, Player>>>,
    /// Track draw submissions: seat -> positions to replace
    pub pending_draws: Arc<Mutex<HashMap<usize, Vec<usize>>>>,
    /// Last showdown results (so all players can see them even after state reset)
    pub last_showdown: Arc<Mutex<Option<String>>>,
}

impl ServerState {
    /// Create a new server state
    pub fn new() -> Self {
        Self {
            table: Arc::new(Mutex::new(None)),
            game_engine: Arc::new(Mutex::new(None)),
            connected_players: Arc::new(Mutex::new(HashMap::new())),
            pending_draws: Arc::new(Mutex::new(HashMap::new())),
            last_showdown: Arc::new(Mutex::new(None)),
        }
    }
}
