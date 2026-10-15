//! Player account type: unique ID, PIN, balance, stats, and history.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::accounts::history::HistoryEvent;
use crate::accounts::stats::PlayerStats;

/// Unique identifier for a player (US 01.01.05). Persists for the session and in storage.
pub type PlayerId = Uuid;

/// Player account with unique ID (US 01.01.05), stats (US 01.01.08), and history for persistence (US 01.01.09).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    /// Unique ID; used to load from storage on reconnect.
    pub id: PlayerId,
    /// PIN for login; used by the server to find the account when the player logs in.
    pub pin: i32,
    /// Display name.
    pub name: String,
    /// Chip balance.
    pub balance: i32,
    /// Rounds played, folds, pots won (US 01.01.08).
    pub stats: PlayerStats,
    /// Audit log: deposits, withdrawals, hand results.
    pub history: Vec<HistoryEvent>,
}

impl Player {
    /// Creates a new player with a unique ID, PIN, and starting balance (US 01.01.05).
    ///
    /// IDs persist for the account and can be restored from storage. The PIN is used to find
    /// the account when the player logs in.
    ///
    /// # Arguments
    ///
    /// * `name` - Display name.
    /// * `pin` - PIN for login (must be unique per account for login lookup).
    /// * `starting_balance` - Initial chip balance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use common::accounts::player::Player;
    /// let p = Player::new("Alice", 1234, 1000);
    /// assert_eq!(p.name, "Alice");
    /// assert_eq!(p.pin, 1234);
    /// assert_eq!(p.balance, 1000);
    /// ```
    pub fn new(name: &str, pin: i32, starting_balance: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            pin,
            name: name.to_string(),
            balance: starting_balance,
            stats: PlayerStats::default(),
            history: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_player_has_unique_id() {
        let a = Player::new("Alice", 1111, 100);
        let b = Player::new("Bob", 0256, 50);
        assert_ne!(a.id, b.id, "each player gets a unique ID (US 01.01.05)");
    }

    #[test]
    fn new_player_sets_name_pin_balance_and_defaults() {
        let p = Player::new("Charlie", 1234, 200);
        assert_eq!(p.name, "Charlie");
        assert_eq!(p.pin, 1234);
        assert_eq!(p.balance, 200);
        assert_eq!(p.stats.rounds_played, 0);
        assert_eq!(p.stats.folds, 0);
        assert_eq!(p.stats.pots_won, 0);
        assert!(p.history.is_empty());
    }
}
