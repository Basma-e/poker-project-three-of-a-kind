//! Persistence layer for player accounts (US 01.01.09).
//!
//! Player accounts are stored in SQLite. Use [`save_player`](storage::save_player) to persist
//! account state, [`load_player`](storage::load_player) to restore an account by ID on reconnect,
//! and [`load_player_by_username_and_pin`](storage::load_player_by_username_and_pin) for login.
//! Additional helpers support lookup by username when required.

pub mod storage;
