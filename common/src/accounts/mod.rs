//! Player accounts: IDs, stats, and history (US 01.01.05, 01.01.08, 01.01.09).
//!
//! This module provides the [`Player`](player::Player) type with unique IDs, optional PIN for
//! login, [`PlayerStats`](stats::PlayerStats) for rounds/folds/pots won, and
//! [`HistoryEvent`](history::HistoryEvent) for auditing.

pub mod history;
pub mod player;
pub mod stats;
