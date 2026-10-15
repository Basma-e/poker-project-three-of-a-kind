//! SQLite-backed persistence for player accounts (US 01.01.09).
//!
//! The database file is created at `data/accounts.db` (relative to process cwd). The schema
//! is applied automatically on first use.  Accounts store balance, stats, and history.
//! Login can be supported by username + PIN

use std::fs;
use std::path::Path;

use common::accounts::history::HistoryEvent;
use common::accounts::player::{Player, PlayerId};
use common::accounts::stats::PlayerStats;
use rusqlite::Connection;
use rusqlite::params;

#[cfg(test)]
use chrono::Utc;

/// Default path for the SQLite database (relative to process cwd).
const DB_PATH: &str = "data/accounts.db";

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    balance INTEGER NOT NULL,
    pin INTEGER NOT NULL,
    rounds_played INTEGER NOT NULL,
    folds INTEGER NOT NULL,
    pots_won INTEGER NOT NULL,
    history TEXT NOT NULL
);
";

/// Returns the filesystem path where the accounts database is stored.
fn db_path() -> &'static Path {
    Path::new(DB_PATH)
}

/// Opens the SQLite connection and ensures the schema exists.
///
/// Creates the `data/` directory if missing, opens (or creates) the DB file,
/// and applies the schema on first use.
fn open_connection() -> Result<Connection, String> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create data dir: {}", e))?;
    }
    let conn = Connection::open(path).map_err(|e| format!("open database: {}", e))?;
    conn.execute_batch(SCHEMA)
        .map_err(|e| format!("init schema: {}", e))?;
    Ok(conn)
}

/// Saves the player account to SQLite (US 01.01.09).
///
/// Upserts by `id`: inserts a new row or updates the existing one. Call after balance or
/// stats change, or on disconnect.
///
/// # Errors
///
/// Returns an error if the database cannot be opened or the write fails.
///
/// # Examples
///
/// ```no_run
/// # use common::accounts::player::Player;
/// # use poker::persistence::storage::save_player;
/// let player = Player::new("Alice", 1234, 500);
/// save_player(&player).expect("save");
/// ```
pub fn save_player(player: &Player) -> Result<(), String> {
    let conn = open_connection()?;
    let history_json =
        serde_json::to_string(&player.history).map_err(|e| format!("serialize history: {}", e))?;
    conn.execute(
        "INSERT INTO accounts (id, name, balance, pin, rounds_played, folds, pots_won, history)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             balance = excluded.balance,
             pin = excluded.pin,
             rounds_played = excluded.rounds_played,
             folds = excluded.folds,
             pots_won = excluded.pots_won,
             history = excluded.history",
        params![
            player.id.to_string(),
            player.name,
            player.balance,
            player.pin,
            player.stats.rounds_played,
            player.stats.folds,
            player.stats.pots_won,
            history_json,
        ],
    )
    .map_err(|e| format!("save player: {}", e))?;
    Ok(())
}

/// Converts a database row into a `Player`.
///
/// Returns `None` if any required field is missing/invalid or history JSON fails to parse.
/// This keeps the public load APIs simple: either you get a valid `Player`, or you get `None`.
fn row_to_player(row: &rusqlite::Row) -> Option<Player> {
    let id_str: String = row.get(0).ok()?;
    let player_id: PlayerId = id_str.parse().ok()?;
    let name: String = row.get(1).ok()?;
    let balance: i32 = row.get(2).ok()?;
    let pin: i32 = row.get(3).ok()?;
    let rounds_played: u32 = row.get(4).ok()?;
    let folds: u32 = row.get(5).ok()?;
    let pots_won: u32 = row.get(6).ok()?;
    let history_json: String = row.get(7).ok()?;
    let history: Vec<HistoryEvent> = serde_json::from_str(&history_json).ok()?;
    Some(Player {
        id: player_id,
        pin,
        name,
        balance,
        stats: PlayerStats {
            rounds_played,
            folds,
            pots_won,
        },
        history,
    })
}

/// Loads a player from SQLite by ID (US 01.01.09).
///
/// Use on reconnect when you have the player's ID; restores balance, stats, and history.
///
/// # Returns
///
/// * `Some(player)` if a row exists and deserialization succeeds.
/// * `None` if no row exists or data is invalid.
pub fn load_player(id: PlayerId) -> Option<Player> {
    let conn = open_connection().ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, balance, pin, rounds_played, folds, pots_won, history
             FROM accounts WHERE id = ?1",
        )
        .ok()?;
    let mut rows = stmt.query(params![id.to_string()]).ok()?;
    let row = rows.next().ok()??;
    row_to_player(&row)
}

/// Loads a player from SQLite by username (name).
/// Used for validating uniqueness during signup and for looking up an account by username.
///
/// # Returns
/// * `Some(player)` if a matching username exists.
/// * `None` if no matching username exists or data is invalid.
pub fn load_player_by_username(username: &str) -> Option<Player> {
    let conn = open_connection().ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, balance, pin, rounds_played, folds, pots_won, history
             FROM accounts WHERE name = ?1",
        )
        .ok()?;
    let mut rows = stmt.query(params![username]).ok()?;
    let row = rows.next().ok()??;
    row_to_player(&row)
}

/// Loads a player from SQLite by username + PIN.
/// This is the preferred authentication lookup for LOGIN since it binds the PIN
/// to a specific username.
///
/// # Returns
/// * `Some(player)` if the username + PIN pair matches an account.
/// * `None` if no match exists or stored data is invalid.
pub fn load_player_by_username_and_pin(username: &str, pin: i32) -> Option<Player> {
    let conn = open_connection().ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, balance, pin, rounds_played, folds, pots_won, history
             FROM accounts WHERE name = ?1 AND pin = ?2",
        )
        .ok()?;
    let mut rows = stmt.query(params![username, pin]).ok()?;
    let row = rows.next().ok()??;
    row_to_player(&row)
}

#[cfg(test)]
mod tests {
    use common::accounts::history::HistoryEvent;
    use common::accounts::player::Player;
    use rusqlite::Connection;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn save_and_load_restores_balance_and_stats() {
        let mut player = Player::new("TestPlayer", 0, 500);
        player.stats.record_round();
        player.stats.record_fold();
        player.stats.record_pot_win();
        player.history.push(HistoryEvent::Deposit {
            amount: 100,
            timestamp: Utc::now(),
        });

        save_player(&player).expect("save should succeed");
        let loaded = load_player(player.id).expect("load should return Some after save");
        assert_eq!(loaded.id, player.id);
        assert_eq!(loaded.name, player.name);
        assert_eq!(loaded.balance, player.balance);
        assert_eq!(loaded.stats.rounds_played, player.stats.rounds_played);
        assert_eq!(loaded.stats.folds, player.stats.folds);
        assert_eq!(loaded.stats.pots_won, player.stats.pots_won);
        assert_eq!(loaded.history.len(), player.history.len());

        // Clean up: remove the test row so test is self-contained
        let path = db_path();
        if path.exists() {
            let conn = Connection::open(path).unwrap();
            conn.execute(
                "DELETE FROM accounts WHERE id = ?1",
                params![player.id.to_string()],
            )
            .unwrap();
        }
    }

    #[test]
    fn load_player_returns_none_for_missing_id() {
        let missing_id = Uuid::new_v4();
        assert!(load_player(missing_id).is_none());
    }

    #[test]
    fn load_player_by_username_and_pin_finds_account() {
        let player = Player::new("PinUser", 1234, 100);
        save_player(&player).expect("save should succeed");

        let loaded = load_player_by_username_and_pin("PinUser", 1234)
            .expect("load by username+pin should return Some");

        assert_eq!(loaded.id, player.id);
        assert_eq!(loaded.pin, 1234);
        assert_eq!(loaded.name, player.name);
        assert_eq!(loaded.balance, player.balance);

        // wrong pin
        assert!(load_player_by_username_and_pin("PinUser", 9999).is_none());
        // wrong username
        assert!(load_player_by_username_and_pin("Nope", 1234).is_none());

        // cleanup
        let path = db_path();
        if path.exists() {
            let conn = Connection::open(path).unwrap();
            conn.execute(
                "DELETE FROM accounts WHERE id = ?1",
                params![player.id.to_string()],
            )
            .unwrap();
        }
    }
}
