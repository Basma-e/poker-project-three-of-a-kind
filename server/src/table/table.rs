//! Table creation and player limits (US 01.01.01): unique ID, 2–5 players.

use common::accounts::player::Player;
use common::poker::variant::GameVariant;
use uuid::Uuid;

use super::state::TableState;

pub type TableId = Uuid;

/// Poker table with unique ID, seated players, and game state.
pub struct Table {
    /// Unique identifier for the table
    pub id: TableId,
    /// Players currently seated at the table
    pub players: Vec<Player>,
    /// Game flow state (betting rounds, turn order)
    pub state: TableState,
    /// Selected variant for next/current hand.
    pub variant: GameVariant,
    /// True after a player explicitly picks table variant.
    pub variant_locked: bool,
}

impl Table {
    /// Create a table with unique ID and player limits (US 01.01.01).
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            players: Vec::new(),
            state: TableState::waiting_for_players(),
            variant: GameVariant::FiveCardDraw,
            variant_locked: false,
        }
    }

    /// Enforce 2–5 players: reject if table full, or duplicate player ID or name.
    pub fn add_player(&mut self, player: Player) -> Result<(), String> {
        if self.players.len() >= 5 {
            return Err("Table is full".into());
        }
        if self.players.iter().any(|p| p.id == player.id) {
            return Err("Player already at table".into());
        }
        if self.players.iter().any(|p| p.name == player.name) {
            return Err("Player name already at table".into());
        }
        self.players.push(player);
        Ok(())
    }

    /// True when 2–5 players are seated and a round can be started (US 01.01.01).
    pub fn can_start(&self) -> bool {
        let n = self.players.len();
        n >= 2 && n <= 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::state::GamePhase;
    use common::accounts::player::Player;

    fn make_player(name: &str, balance: i32) -> Player {
        Player::new(name, 0, balance)
    }

    #[test]
    fn new_table_has_unique_id_and_no_players() {
        let t = Table::new();
        assert_eq!(t.players.len(), 0);
        assert_eq!(t.state.phase, GamePhase::WaitingForPlayers);
        assert_eq!(t.variant, GameVariant::FiveCardDraw);
        assert!(!t.variant_locked);
    }

    #[test]
    fn can_start_false_with_0_or_1_players() {
        let mut t = Table::new();
        assert!(!t.can_start());
        t.add_player(make_player("Alice", 100)).unwrap();
        assert!(!t.can_start());
    }

    #[test]
    fn can_start_true_with_2_to_5_players() {
        let mut t = Table::new();
        t.add_player(make_player("Alice", 100)).unwrap();
        t.add_player(make_player("Bob", 100)).unwrap();
        assert!(t.can_start());
        t.add_player(make_player("Carol", 100)).unwrap();
        t.add_player(make_player("Dave", 100)).unwrap();
        t.add_player(make_player("Eve", 100)).unwrap();
        assert!(t.can_start());
    }

    #[test]
    fn add_player_rejects_when_full() {
        let mut t = Table::new();
        for i in 0..5 {
            t.add_player(make_player(&format!("P{}", i), 100)).unwrap();
        }
        let err = t.add_player(make_player("Sixth", 100)).unwrap_err();
        assert!(err.contains("full"));
    }

    #[test]
    fn add_player_rejects_duplicate_name() {
        let mut t = Table::new();
        let p = make_player("Alice", 100);
        t.add_player(p).unwrap();
        let p2 = make_player("Alice", 50);
        let err = t.add_player(p2).unwrap_err();
        assert!(err.contains("name"));
    }

    #[test]
    fn add_player_rejects_duplicate_id() {
        let mut t = Table::new();
        let p = make_player("Alice", 100);
        let id = p.id;
        t.add_player(p).unwrap();
        let mut p2 = make_player("Bob", 50);
        p2.id = id;
        let err = t.add_player(p2).unwrap_err();
        assert!(err.contains("already at table"));
    }
}
