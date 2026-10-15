//! Betting logic (US 01.01.03): bet, call, raise, fold, pot updates and turn order

use crate::rules::rules::HouseRules;
use crate::table::state::{GamePhase, TableState};
use chrono::Utc;
use common::accounts::history::HistoryEvent;
use common::accounts::player::Player;

// Represents all possible betting actions a player can take
// Bet and Raise store an amount
#[derive(Debug, Clone, Copy)]
pub enum BetAction {
    Bet(i32),
    Call,
    Raise(i32),
    Fold,
    Check, // Pass when no bet to call
}

/// BettingRound owns the house rules used to validate bets
pub struct BettingRound {
    pub rules: HouseRules,
}

impl BettingRound {
    /// Create a betting round with house rules.
    pub fn new(rules: HouseRules) -> Self {
        Self { rules }
    }

    // Applies one betting action for the current player
    // Mutates table state and player balances accordingly
    pub fn apply_action(
        &self,
        state: &mut TableState,
        players: &mut [Player],
        acting_seat: usize,
        action: BetAction,
    ) -> Result<(), String> {
        // Reject actions if the game is not currently in a betting phase
        if state.phase != GamePhase::BettingRound {
            return Err("not in betting round".into());
        }
        // Cache number of players at the table
        let player_count = players.len();
        // Ensure seat index is valid
        if acting_seat >= player_count {
            return Err("invalid seat".into());
        }
        // Ensure betting state matches number of players
        if state.player_bet_states.len() != player_count {
            return Err("player count mismatch".into());
        }
        // Enforce strict turn order
        if state.current_actor_seat != acting_seat {
            return Err("not your turn".into());
        }
        // Prevent folded players from acting
        if state.player_bet_states[acting_seat].folded {
            return Err("already folded".into());
        }

        // Dispatch to the correct handler based on action type
        match action {
            BetAction::Bet(amount) => self.apply_bet(state, players, acting_seat, amount)?,
            BetAction::Call => self.apply_call(state, players, acting_seat)?,
            BetAction::Raise(amount) => self.apply_raise(state, players, acting_seat, amount)?,
            BetAction::Fold => self.apply_fold(state, players, acting_seat),
            BetAction::Check => self.apply_check(state, players, acting_seat)?,
        }

        // Move turn to the next active player
        // Note: is_betting_round_complete() checks AFTER advance, so it correctly detects
        // when we've come back around to the completion seat
        state.advance_to_next_actor(player_count);
        Ok(())
    }

    // Handles opening bet when no bet exists yet
    fn apply_bet(
        &self,
        state: &mut TableState,
        players: &mut [Player],
        acting_seat: usize,
        amount: i32,
    ) -> Result<(), String> {
        // Prevent betting if a bet already exists
        if state.current_bet_to_call != 0 {
            return Err("cannot bet when a bet is already in place; use raise or call".into());
        }
        // Enforce min/max bet limits
        if !self.rules.validate_bet(amount) {
            return Err(format!(
                "bet {} outside house limits [{}, {}]",
                amount, self.rules.min_bet, self.rules.max_bet
            ));
        }
        // Player must have enough chips to bet
        if players[acting_seat].balance < amount {
            return Err("insufficient balance".into());
        }
        // Set bet to call, update pot, record contribution, deduct balance
        state.current_bet_to_call = amount;
        state.initial_bettor_seat = Some(acting_seat); // Track who made the initial bet
        state.pot += amount;
        state.player_bet_states[acting_seat].contribution_this_round = amount;
        players[acting_seat].balance -= amount;
        // Record history event
        players[acting_seat].history.push(HistoryEvent::Withdraw {
            amount,
            timestamp: Utc::now(),
        });
        Ok(())
    }

    fn apply_call(
        &self,
        state: &mut TableState,
        players: &mut [Player],
        acting_seat: usize,
    ) -> Result<(), String> {
        // Calculate how much more the player must contribute to match the bet
        let need = state.current_bet_to_call
            - state.player_bet_states[acting_seat].contribution_this_round;
        // Cannot call if already matched
        if need <= 0 {
            return Err("nothing to call".into());
        }
        // Ensure player can afford the call
        if players[acting_seat].balance < need {
            return Err("insufficient balance to call".into());
        }
        // Add chips to pot and update player state
        state.pot += need;
        state.player_bet_states[acting_seat].contribution_this_round += need;
        players[acting_seat].balance -= need;
        // Record history event
        players[acting_seat].history.push(HistoryEvent::Withdraw {
            amount: need,
            timestamp: Utc::now(),
        });
        Ok(())
    }

    fn apply_raise(
        &self,
        state: &mut TableState,
        players: &mut [Player],
        acting_seat: usize,
        amount: i32,
    ) -> Result<(), String> {
        // Raise must increase the current bet
        if amount <= state.current_bet_to_call {
            return Err("raise must be greater than current bet to call".into());
        }
        // Enforce house bet limits
        if !self.rules.validate_bet(amount) {
            return Err(format!(
                "raise {} outside house limits [{}, {}]",
                amount, self.rules.min_bet, self.rules.max_bet
            ));
        }
        // Enforce maximum raises per betting round
        if state.raises_this_round >= self.rules.max_raises_per_round {
            return Err("raise limit reached for this round".into());
        }
        // Amount the raiser still needs to put in
        let need = amount - state.player_bet_states[acting_seat].contribution_this_round;
        if players[acting_seat].balance < need {
            return Err("insufficient balance to raise".into());
        }

        // Update bet target and track last raiser for round completion
        state.current_bet_to_call = amount;
        state.last_raiser_seat = Some(acting_seat);
        state.pot += need;
        state.player_bet_states[acting_seat].contribution_this_round = amount;
        players[acting_seat].balance -= need;
        state.raises_this_round += 1;
        // Record history event
        players[acting_seat].history.push(HistoryEvent::Withdraw {
            amount: need,
            timestamp: Utc::now(),
        });

        Ok(())
    }

    // Mark player as folded for this betting round (US 01.01.08: track folds)
    fn apply_fold(&self, state: &mut TableState, players: &mut [Player], acting_seat: usize) {
        state.player_bet_states[acting_seat].folded = true;
        players[acting_seat].stats.record_fold();
    }

    // Handles check action: pass when no bet to call
    fn apply_check(
        &self,
        state: &mut TableState,
        _players: &mut [Player],
        _acting_seat: usize,
    ) -> Result<(), String> {
        // Can only check if there's no bet to call
        if state.current_bet_to_call > 0 {
            return Err(
                "cannot check when there is a bet to call; use call, raise, or fold".into(),
            );
        }
        // Check is a no-op (no chips moved, just pass turn)
        Ok(())
    }
}

// Unit tests verifying betting behavior and rule enforcement
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::rules::HouseRules;
    use crate::table::state::TableState;
    use common::accounts::player::Player;

    fn make_player(name: &str, balance: i32) -> Player {
        Player::new(name, 0, balance)
    }

    #[test]
    fn bet_then_call_pot_and_balances() {
        let rules = HouseRules::new(1, 100, 4, 5, 1, 2);
        let round = BettingRound::new(rules);
        let mut state = TableState::waiting_for_players();
        let mut players = vec![make_player("A", 100), make_player("B", 100)];
        state.start_betting_round(2, 0); // dealer 0, first to act is 1
        assert_eq!(state.current_actor_seat, 1);
        round
            .apply_action(&mut state, &mut players, 1, BetAction::Bet(10))
            .unwrap();
        assert_eq!(state.pot, 10);
        assert_eq!(state.current_bet_to_call, 10);
        assert_eq!(players[1].balance, 90);
        assert_eq!(state.current_actor_seat, 0);
        round
            .apply_action(&mut state, &mut players, 0, BetAction::Call)
            .unwrap();
        assert_eq!(state.pot, 20);
        assert_eq!(players[0].balance, 90);
        assert!(state.is_betting_round_complete());
    }

    #[test]
    fn not_your_turn_rejected() {
        let rules = HouseRules::new(1, 100, 4, 5, 1, 2);
        let round = BettingRound::new(rules);
        let mut state = TableState::waiting_for_players();
        let mut players = vec![make_player("A", 100), make_player("B", 100)];
        state.start_betting_round(2, 0);
        let err = round
            .apply_action(&mut state, &mut players, 0, BetAction::Bet(10))
            .unwrap_err();
        assert!(err.contains("not your turn"));
    }

    #[test]
    fn bet_below_min_rejected() {
        let rules = HouseRules::new(10, 100, 4, 5, 1, 2);
        let round = BettingRound::new(rules);
        let mut state = TableState::waiting_for_players();
        let mut players = vec![make_player("A", 100), make_player("B", 100)];
        state.start_betting_round(2, 0);
        let err = round
            .apply_action(&mut state, &mut players, 1, BetAction::Bet(5))
            .unwrap_err();
        assert!(err.contains("outside house limits"));
    }

    #[test]
    fn insufficient_balance_rejected() {
        let rules = HouseRules::new(1, 100, 4, 5, 1, 2);
        let round = BettingRound::new(rules);
        let mut state = TableState::waiting_for_players();
        // A has 5, B has 100. B bets 10; A cannot call (needs 10, has 5).
        let mut players = vec![make_player("A", 5), make_player("B", 100)];
        state.start_betting_round(2, 0);
        round
            .apply_action(&mut state, &mut players, 1, BetAction::Bet(10))
            .unwrap();
        let err = round
            .apply_action(&mut state, &mut players, 0, BetAction::Call)
            .unwrap_err();
        assert!(err.contains("insufficient balance"));
    }

    #[test]
    fn fold_then_round_complete() {
        let rules = HouseRules::new(1, 100, 4, 5, 1, 2);
        let round = BettingRound::new(rules);
        let mut state = TableState::waiting_for_players();
        let mut players = vec![make_player("A", 100), make_player("B", 100)];
        state.start_betting_round(2, 0);
        round
            .apply_action(&mut state, &mut players, 1, BetAction::Fold)
            .unwrap();
        assert!(state.player_bet_states[1].folded);
        assert!(state.is_betting_round_complete());
    }

    #[test]
    fn raise_limit_enforced() {
        let rules = HouseRules::new(1, 100, 1, 5, 1, 2); // only 1 raise per round
        let round = BettingRound::new(rules);
        let mut state = TableState::waiting_for_players();
        let mut players = vec![make_player("A", 200), make_player("B", 200)];
        state.start_betting_round(2, 0);
        round
            .apply_action(&mut state, &mut players, 1, BetAction::Bet(10))
            .unwrap();
        round
            .apply_action(&mut state, &mut players, 0, BetAction::Raise(20))
            .unwrap();
        let err = round
            .apply_action(&mut state, &mut players, 1, BetAction::Raise(30))
            .unwrap_err();
        assert!(err.contains("raise limit"));
    }
}
