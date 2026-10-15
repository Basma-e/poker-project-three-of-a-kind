//! History events for player accounts: deposits, withdrawals, hand results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Major account events for auditing and display (US 01.01.08 / 01.01.09).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistoryEvent {
    /// Chips added to the account (e.g. buy-in).
    Deposit {
        /// Amount added.
        amount: i32,
        timestamp: DateTime<Utc>,
    },
    /// Chips removed from the account.
    Withdraw {
        /// Amount removed.
        amount: i32,
        timestamp: DateTime<Utc>,
    },
    /// Player won a pot; can be pushed when awarding the pot at showdown.
    HandResult {
        /// Pot amount won.
        pot_won: i32,
        timestamp: DateTime<Utc>,
        /// Poker variant/game type played for this hand (if known).
        ///
        /// Backwards compatible with older stored accounts where this field
        /// did not exist.
        #[serde(default)]
        game_type: Option<String>,
    },
}
