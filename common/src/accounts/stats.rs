//! Player statistics: rounds played, folds, pots won (US 01.01.08).

use serde::{Deserialize, Serialize};

/// Per-player statistics for performance tracking (US 01.01.08).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    /// Number of hands (rounds) the player has participated in.
    pub rounds_played: u32,
    /// Number of times the player has folded.
    pub folds: u32,
    /// Number of pots the player has won.
    pub pots_won: u32,
}

impl PlayerStats {
    /// Increments rounds played. Call when a hand/round completes for this player.
    pub fn record_round(&mut self) {
        self.rounds_played += 1;
    }

    /// Increments folds. Call when the player folds (e.g. in betting).
    pub fn record_fold(&mut self) {
        self.folds += 1;
    }

    /// Increments pots won. Call when the player wins the pot (e.g. at showdown).
    pub fn record_pot_win(&mut self) {
        self.pots_won += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_stats_are_zero() {
        let s = PlayerStats::default();
        assert_eq!(s.rounds_played, 0);
        assert_eq!(s.folds, 0);
        assert_eq!(s.pots_won, 0);
    }

    #[test]
    fn record_round_increments_rounds_played() {
        let mut s = PlayerStats::default();
        s.record_round();
        assert_eq!(s.rounds_played, 1);
        s.record_round();
        s.record_round();
        assert_eq!(s.rounds_played, 3);
    }

    #[test]
    fn record_fold_increments_folds() {
        let mut s = PlayerStats::default();
        s.record_fold();
        assert_eq!(s.folds, 1);
        s.record_fold();
        assert_eq!(s.folds, 2);
    }

    #[test]
    fn record_pot_win_increments_pots_won() {
        let mut s = PlayerStats::default();
        s.record_pot_win();
        assert_eq!(s.pots_won, 1);
        s.record_pot_win();
        s.record_pot_win();
        assert_eq!(s.pots_won, 3);
    }
}
