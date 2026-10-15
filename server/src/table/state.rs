//! Game flow and state machine for table (US 01.01.01 / 01.01.03).
//! Phase 1: Five-Card Draw, BettingRound is where bet/call/raise/fold apply.

/// Game phase for the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    /// No game in progress; waiting for 2–5 players
    WaitingForPlayers,
    /// Players are currently betting
    BettingRound,
    /// Five-Card Draw phase (players discard and draw cards)
    Draw,
    /// Hands are revealed and winner is determined
    Showdown,
    /// Cleanup/reset state between hands
    BetweenRounds,
}

/// Tracks per-player betting info during a betting round
#[derive(Debug, Clone, Default)]
pub struct PlayerBetState {
    pub folded: bool,                 // True if player has folded this round
    pub contribution_this_round: i32, // Amount this player has put into the pot this betting round
}

/// Table state: current phase, betting data, turn order (dealer + current actor).
#[derive(Debug, Clone)]
pub struct TableState {
    pub phase: GamePhase,
    /// Main pot (sum of contributions this hand).
    pub pot: i32,
    /// Current bet to call (0 if no bet yet this round).
    pub current_bet_to_call: i32,
    /// Dealer seat index; acting order is dealer+1, dealer+2, ... wrapping.
    pub dealer_seat: usize,
    /// Seat index of player who must act next (only valid in BettingRound).
    pub current_actor_seat: usize,
    /// First to act this round (for detecting round complete: back to first and everyone matched).
    pub first_actor_seat: usize,
    /// Per-seat state: folded and contribution this round. Length = number of players at table.
    pub player_bet_states: Vec<PlayerBetState>,
    /// Number of raises made this betting round (for house rule limit).
    pub raises_this_round: u32,
    /// Seat index of the most recent raiser (None if no raise yet)
    pub last_raiser_seat: Option<usize>,
    /// Seat index of the player who made the initial bet (None if no bet yet, only checks/raises)
    pub initial_bettor_seat: Option<usize>,
}

impl TableState {
    /// New state with no game in progress (waiting for players).
    pub fn waiting_for_players() -> Self {
        Self {
            phase: GamePhase::WaitingForPlayers,
            pot: 0,
            current_bet_to_call: 0,
            dealer_seat: 0,
            current_actor_seat: 0,
            first_actor_seat: 0,
            player_bet_states: Vec::new(),
            raises_this_round: 0,
            last_raiser_seat: None,
            initial_bettor_seat: None,
        }
    }

    /// Start a betting round: reset per-player contributions for this round, set phase and acting order.
    /// `player_count`: number of players in the hand (seats that are active).
    /// Preserves folded states from previous betting rounds WITHIN THE SAME HAND (folded players stay folded).
    /// If `player_bet_states` is empty, this is a new hand and all players start fresh.
    pub fn start_betting_round(&mut self, player_count: usize, dealer_seat: usize) {
        let first_actor = (dealer_seat + 1) % player_count;
        self.start_betting_round_with_first_actor(player_count, dealer_seat, first_actor);
    }

    /// Start a betting round with explicit first-actor seat.
    pub fn start_betting_round_with_first_actor(
        &mut self,
        player_count: usize,
        dealer_seat: usize,
        first_actor: usize,
    ) {
        self.phase = GamePhase::BettingRound;
        self.current_bet_to_call = 0;
        self.raises_this_round = 0;
        self.last_raiser_seat = None;
        self.initial_bettor_seat = None;
        self.dealer_seat = dealer_seat;

        // Preserve folded states from previous rounds WITHIN THE SAME HAND
        // If player_bet_states is empty, this is a new hand - all players start fresh
        let old_folded_states: Vec<bool> = if self.player_bet_states.is_empty() {
            Vec::new() // New hand - no folded states to preserve
        } else {
            self.player_bet_states.iter().map(|s| s.folded).collect()
        };

        // Create new bet states, preserving folded status only if within same hand
        self.player_bet_states = (0..player_count)
            .map(|i| {
                let was_folded = i < old_folded_states.len() && old_folded_states[i];
                PlayerBetState {
                    folded: was_folded,
                    contribution_this_round: 0, // Reset contribution for new round
                }
            })
            .collect();

        // First to act is configurable by variant/street.
        self.first_actor_seat = first_actor % player_count;
        self.current_actor_seat = self.first_actor_seat;

        // Skip to first non-folded player
        while self.current_actor_seat < self.player_bet_states.len()
            && self.player_bet_states[self.current_actor_seat].folded
        {
            self.current_actor_seat = (self.current_actor_seat + 1) % player_count;
            if self.current_actor_seat == self.first_actor_seat {
                // All players folded (shouldn't happen, but handle gracefully)
                break;
            }
        }
    }

    /// Advance current actor to next non-folded seat. Returns true when we've gone
    /// full circle (everyone still in has had a chance to act).
    /// Never leaves current_actor_seat on a folded player when returning false (next to act).
    pub fn advance_to_next_actor(&mut self, player_count: usize) -> bool {
        if self.active_player_count() <= 1 {
            return true;
        }

        loop {
            self.current_actor_seat = (self.current_actor_seat + 1) % player_count;
            if self.current_actor_seat == self.first_actor_seat {
                // We've gone full circle.
                if !self.player_bet_states[self.current_actor_seat].folded {
                    return true; // First actor can still act => round complete
                }
                // First actor folded: full circle is still complete. Set current to completion
                // seat so is_betting_round_complete() can return true (we may transition phase next).
                let completion_seat = match self.last_raiser_seat {
                    Some(seat) => seat,
                    None => match self.initial_bettor_seat {
                        Some(seat) => seat,
                        None => self.first_actor_seat,
                    },
                };
                self.current_actor_seat = completion_seat;
                return true;
            }
            if !self.player_bet_states[self.current_actor_seat].folded {
                return false; // Next player to act
            }
        }
    }

    /// Check if betting round is complete: only one player left, or we've gone full circle
    /// (current actor back to completion seat) and all non-folded have contributed at least current_bet_to_call.
    ///
    /// Note: This is called AFTER advance_to_next_actor(), so current_actor_seat has already moved.
    /// If we're back at the completion seat, it means we've completed a full cycle.
    pub fn is_betting_round_complete(&self) -> bool {
        if self.active_player_count() <= 1 {
            return true;
        }

        // Determine the seat that marks completion
        // If someone raised, we need to get back to them
        // If someone bet (but didn't raise), we need to get back to the bettor
        // Otherwise, we need to get back to first actor (everyone checked)
        let completion_seat = match self.last_raiser_seat {
            Some(seat) => seat, // Someone raised - must get back to raiser
            None => match self.initial_bettor_seat {
                Some(seat) => seat,            // Someone bet - must get back to bettor
                None => self.first_actor_seat, // No bet/raise - get back to first actor
            },
        };

        // After advancing, if we're back at the completion seat, check if everyone has matched
        if self.current_actor_seat != completion_seat {
            return false;
        }

        // Special case: If we're back at the completion seat, check if that player has already matched
        // This handles both raisers and first actors who bet
        let completion_player_contribution =
            self.player_bet_states[completion_seat].contribution_this_round;
        if completion_player_contribution >= self.current_bet_to_call {
            // Completion player has already matched (they bet/raised), check if everyone else has matched
            let all_matched = self
                .player_bet_states
                .iter()
                .enumerate()
                .all(|(seat, state)| {
                    seat == completion_seat || // Skip completion player (already checked)
                    state.folded ||
                    state.contribution_this_round >= self.current_bet_to_call
                });
            if all_matched {
                return true;
            }
        }

        // All active players must have matched the current bet
        self.player_bet_states
            .iter()
            .all(|state| state.folded || state.contribution_this_round >= self.current_bet_to_call)
    }

    /// Count of active (non-folded) players this round.
    pub fn active_player_count(&self) -> usize {
        self.player_bet_states.iter().filter(|s| !s.folded).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_for_players_phase() {
        let s = TableState::waiting_for_players();
        assert_eq!(s.phase, GamePhase::WaitingForPlayers);
        assert_eq!(s.pot, 0);
        assert!(s.player_bet_states.is_empty());
    }

    #[test]
    fn start_betting_round_sets_first_actor() {
        let mut s = TableState::waiting_for_players();
        s.start_betting_round(3, 0); // dealer seat 0, 3 players
        assert_eq!(s.phase, GamePhase::BettingRound);
        assert_eq!(s.current_bet_to_call, 0);
        assert_eq!(s.dealer_seat, 0);
        assert_eq!(s.first_actor_seat, 1); // left of dealer
        assert_eq!(s.current_actor_seat, 1);
        assert_eq!(s.player_bet_states.len(), 3);
        assert_eq!(s.raises_this_round, 0);
    }

    #[test]
    fn advance_to_next_actor_wraps() {
        let mut s = TableState::waiting_for_players();
        s.start_betting_round(3, 0);
        let _ = s.advance_to_next_actor(3);
        assert_eq!(s.current_actor_seat, 2);
        let _ = s.advance_to_next_actor(3);
        assert_eq!(s.current_actor_seat, 0);
        let done = s.advance_to_next_actor(3);
        assert!(done);
        assert_eq!(s.current_actor_seat, 1);
    }

    #[test]
    fn is_betting_round_complete_single_player() {
        let mut s = TableState::waiting_for_players();
        s.start_betting_round(2, 0);
        s.player_bet_states[0].folded = true;
        assert!(s.is_betting_round_complete());
    }

    #[test]
    fn is_betting_round_complete_all_matched() {
        let mut s = TableState::waiting_for_players();
        s.start_betting_round(2, 0);
        s.current_bet_to_call = 10;
        s.player_bet_states[0].contribution_this_round = 10;
        s.player_bet_states[1].contribution_this_round = 10;
        s.current_actor_seat = s.first_actor_seat;
        assert!(s.is_betting_round_complete());
    }
}
