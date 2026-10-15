//! Game orchestrator for supported poker variants.
//! Draw: deal -> betting -> draw -> betting -> showdown
//! Hold'em: blinds -> hole cards -> preflop/flop/turn/river betting -> showdown

use chrono::Utc;
use common::accounts::history::HistoryEvent;
use common::poker::card::Card;
use common::poker::deck::Deck;
use common::poker::draw::{deal_hands_round_robin, replace_cards};
use common::poker::hand::Hand;
use common::poker::hand_eval::{
    EvaluatedHand, compare_hands_full, evaluate_best_five_from_seven, evaluate_hand,
};
use common::poker::variant::GameVariant;
use std::collections::HashMap;

use crate::betting::betting::{BetAction, BettingRound};
use crate::persistence::storage::save_player;
use crate::rules::rules::HouseRules;
use crate::table::state::GamePhase;
use crate::table::table::Table;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldemStreet {
    PreFlop,
    Flop,
    Turn,
    River,
}

/// Game orchestrator that manages a complete hand.
pub struct GameEngine {
    /// House rules for betting limits
    pub rules: HouseRules,
    /// Current deck (shuffled for each hand)
    deck: Deck,
    /// Draw hands: seat index -> 5-card hand.
    draw_hands: HashMap<usize, Hand>,
    /// Hold'em private hole cards: seat index -> 2 cards.
    holdem_hole_cards: HashMap<usize, [Card; 2]>,
    /// Hold'em community cards.
    holdem_community_cards: Vec<Card>,
    /// Hold'em current street.
    holdem_street: Option<HoldemStreet>,
    /// Track if we've been through draw phase in current hand
    has_been_through_draw: bool,
}

impl GameEngine {
    /// Create a new game engine with house rules.
    pub fn new(rules: HouseRules) -> Self {
        Self {
            rules,
            deck: Deck::new(),
            draw_hands: HashMap::new(),
            holdem_hole_cards: HashMap::new(),
            holdem_community_cards: Vec::new(),
            holdem_street: None,
            has_been_through_draw: false,
        }
    }

    pub fn current_variant(&self, table: &Table) -> GameVariant {
        table.variant
    }

    pub fn holdem_street(&self) -> Option<HoldemStreet> {
        self.holdem_street
    }

    pub fn holdem_community_cards(&self) -> &[Card] {
        &self.holdem_community_cards
    }

    /// Check if we've been through the draw phase in the current hand
    pub fn has_been_through_draw(&self) -> bool {
        self.has_been_through_draw
    }

    /// Collect ante from all players before dealing.
    /// Returns error if any player cannot afford the ante.
    fn collect_ante(&self, table: &mut Table) -> Result<(), String> {
        let ante = self.rules.ante_amount;
        if ante <= 0 {
            return Ok(());
        }

        // Check all players can afford ante
        for player in &table.players {
            if player.balance < ante {
                return Err(format!(
                    "Player {} cannot afford ante of {}. Current balance: {}",
                    player.name, ante, player.balance
                ));
            }
        }

        // Collect ante from all players
        for player in &mut table.players {
            player.balance -= ante;
            table.state.pot += ante;
            player.history.push(HistoryEvent::Withdraw {
                amount: ante,
                timestamp: Utc::now(),
            });
            save_player(player).map_err(|e| format!("Failed to save player after ante: {}", e))?;
        }

        Ok(())
    }

    fn post_holdem_blinds(&self, table: &mut Table) -> Result<(usize, usize), String> {
        let player_count = table.players.len();
        let dealer = table.state.dealer_seat;
        let small_blind_seat = (dealer + 1) % player_count;
        let big_blind_seat = (dealer + 2) % player_count;
        let sb = self.rules.small_blind;
        let bb = self.rules.big_blind;

        if table.players[small_blind_seat].balance < sb {
            return Err(format!(
                "Player {} cannot afford small blind {}",
                table.players[small_blind_seat].name, sb
            ));
        }
        if table.players[big_blind_seat].balance < bb {
            return Err(format!(
                "Player {} cannot afford big blind {}",
                table.players[big_blind_seat].name, bb
            ));
        }

        table.players[small_blind_seat].balance -= sb;
        table.players[small_blind_seat]
            .history
            .push(HistoryEvent::Withdraw {
                amount: sb,
                timestamp: Utc::now(),
            });
        table.players[big_blind_seat].balance -= bb;
        table.players[big_blind_seat]
            .history
            .push(HistoryEvent::Withdraw {
                amount: bb,
                timestamp: Utc::now(),
            });
        table.state.pot += sb + bb;

        save_player(&table.players[small_blind_seat])
            .map_err(|e| format!("Failed to save small blind player: {}", e))?;
        save_player(&table.players[big_blind_seat])
            .map_err(|e| format!("Failed to save big blind player: {}", e))?;

        Ok((small_blind_seat, big_blind_seat))
    }

    /// Start a new hand: collect ante, deal cards to all players and begin first betting round.
    /// Returns error if table cannot start (not enough players), players can't afford ante, or deck issues.
    pub fn start_hand(&mut self, table: &mut Table) -> Result<(), String> {
        if !table.can_start() {
            return Err("Table needs 2-5 players to start".into());
        }

        // Reset deck and shuffle
        self.deck = Deck::new();
        self.deck.shuffle();
        self.draw_hands.clear();
        self.holdem_hole_cards.clear();
        self.holdem_community_cards.clear();
        self.holdem_street = None;
        self.has_been_through_draw = false;

        // Reset all player bet states for new hand (clear folded states from previous hand)
        let player_count = table.players.len();
        table.state.player_bet_states.clear();

        // Rotate dealer (for next hand, dealer moves left)
        let dealer_seat = (table.state.dealer_seat + 1) % player_count;
        table.state.dealer_seat = dealer_seat;

        match table.variant {
            GameVariant::FiveCardDraw => {
                self.collect_ante(table)?;
                let hands = deal_hands_round_robin(&mut self.deck, player_count)
                    .ok_or("Deck does not have enough cards")?;
                for (seat, hand) in hands.into_iter().enumerate() {
                    self.draw_hands.insert(seat, hand);
                }
                table.state.start_betting_round(player_count, dealer_seat);
            }
            GameVariant::TexasHoldem => {
                let (_sb, bb) = self.post_holdem_blinds(table)?;
                for seat in 0..player_count {
                    let c1 = self
                        .deck
                        .deal_one()
                        .ok_or("Deck ran out while dealing hole cards")?;
                    let c2 = self
                        .deck
                        .deal_one()
                        .ok_or("Deck ran out while dealing hole cards")?;
                    self.holdem_hole_cards.insert(seat, [c1, c2]);
                }
                self.holdem_street = Some(HoldemStreet::PreFlop);
                let first_actor = (bb + 1) % player_count;
                table.state.start_betting_round_with_first_actor(
                    player_count,
                    dealer_seat,
                    first_actor,
                );
                table.state.current_bet_to_call = self.rules.big_blind;
                table.state.player_bet_states[(dealer_seat + 1) % player_count]
                    .contribution_this_round = self.rules.small_blind;
                table.state.player_bet_states[bb].contribution_this_round = self.rules.big_blind;
                table.state.initial_bettor_seat = Some(bb);
            }
        }

        Ok(())
    }

    /// Process a betting action for the current player.
    /// Returns error if action is invalid or not player's turn.
    pub fn process_betting_action(
        &self,
        table: &mut Table,
        action: BetAction,
    ) -> Result<(), String> {
        let betting_round = BettingRound::new(self.rules.clone());
        let acting_seat = table.state.current_actor_seat;
        betting_round.apply_action(&mut table.state, &mut table.players, acting_seat, action)?;

        // Save player after balance change
        save_player(&table.players[acting_seat])
            .map_err(|e| format!("Failed to save player: {}", e))?;

        Ok(())
    }

    /// Process the draw phase: replace cards for players who want to draw.
    /// `draws` maps seat index to indices of cards to replace (empty = stand pat).
    /// Returns error if any draw is invalid.
    pub fn process_draw_phase(
        &mut self,
        table: &mut Table,
        draws: HashMap<usize, Vec<usize>>,
    ) -> Result<(), String> {
        if table.state.phase != GamePhase::Draw {
            return Err("Not in draw phase".into());
        }

        for (seat, replace_idxs) in draws {
            if seat >= table.players.len() {
                return Err(format!("Invalid seat index: {}", seat));
            }
            if !self.draw_hands.contains_key(&seat) {
                return Err(format!("No hand for seat {}", seat));
            }

            if replace_idxs.is_empty() {
                // Player stands pat - no action needed
                continue;
            }

            // Replace cards
            let hand = self.draw_hands.get(&seat).unwrap();
            let new_hand = replace_cards(&mut self.deck, hand, &replace_idxs)
                .ok_or(format!("Failed to replace cards for seat {}", seat))?;
            self.draw_hands.insert(seat, new_hand);
        }

        // Mark that we've been through draw phase
        self.has_been_through_draw = true;

        // Move to second betting round
        let player_count = table.players.len();
        let active_count = table.state.active_player_count();
        if active_count > 1 {
            table
                .state
                .start_betting_round(player_count, table.state.dealer_seat);
        } else {
            // Only one player left - skip to showdown
            table.state.phase = GamePhase::Showdown;
        }

        Ok(())
    }

    /// Transition from first betting round to draw phase.
    /// Call this when first betting round completes.
    /// Sets up turn order for draw phase (same as betting: left of dealer first).
    /// Only non-folded players get a turn to draw; folded players are skipped.
    pub fn transition_to_draw(&mut self, table: &mut Table) {
        if table.variant != GameVariant::FiveCardDraw {
            return;
        }
        table.state.phase = GamePhase::Draw;
        // Set first actor for draw phase (same order as betting: left of dealer)
        let player_count = table.players.len();
        table.state.first_actor_seat = (table.state.dealer_seat + 1) % player_count;
        table.state.current_actor_seat = table.state.first_actor_seat;
        // Skip to first non-folded player (folded players do not get a draw turn)
        while table.state.current_actor_seat < table.state.player_bet_states.len()
            && table.state.player_bet_states[table.state.current_actor_seat].folded
        {
            table.state.current_actor_seat = (table.state.current_actor_seat + 1) % player_count;
            if table.state.current_actor_seat == table.state.first_actor_seat {
                break; // All folded (shouldn't happen after betting round)
            }
        }
    }

    /// Transition from draw/second betting round to showdown.
    /// Call this when second betting round completes or only one player remains.
    pub fn transition_to_showdown(&mut self, table: &mut Table) {
        table.state.phase = GamePhase::Showdown;
    }

    pub fn advance_holdem_after_betting_round(&mut self, table: &mut Table) -> Result<(), String> {
        if table.variant != GameVariant::TexasHoldem {
            return Ok(());
        }
        let street = self
            .holdem_street
            .ok_or("Hold'em street missing during transition")?;
        match street {
            HoldemStreet::PreFlop => {
                self.deck.deal_one(); // burn
                for _ in 0..3 {
                    self.holdem_community_cards.push(
                        self.deck
                            .deal_one()
                            .ok_or("Deck ran out while dealing flop")?,
                    );
                }
                self.holdem_street = Some(HoldemStreet::Flop);
                table
                    .state
                    .start_betting_round(table.players.len(), table.state.dealer_seat);
            }
            HoldemStreet::Flop => {
                self.deck.deal_one(); // burn
                self.holdem_community_cards.push(
                    self.deck
                        .deal_one()
                        .ok_or("Deck ran out while dealing turn")?,
                );
                self.holdem_street = Some(HoldemStreet::Turn);
                table
                    .state
                    .start_betting_round(table.players.len(), table.state.dealer_seat);
            }
            HoldemStreet::Turn => {
                self.deck.deal_one(); // burn
                self.holdem_community_cards.push(
                    self.deck
                        .deal_one()
                        .ok_or("Deck ran out while dealing river")?,
                );
                self.holdem_street = Some(HoldemStreet::River);
                table
                    .state
                    .start_betting_round(table.players.len(), table.state.dealer_seat);
            }
            HoldemStreet::River => {
                table.state.phase = GamePhase::Showdown;
            }
        }
        Ok(())
    }

    /// Determine winner(s) and award pot. Updates player stats and saves to persistence.
    /// Returns list of winning seat indices (for ties, multiple winners split pot).
    pub fn determine_winner_and_award_pot(
        &mut self,
        table: &mut Table,
    ) -> Result<Vec<usize>, String> {
        if table.state.phase != GamePhase::Showdown {
            return Err("Not in showdown phase".into());
        }

        // Get active (non-folded) players and their hands
        let active_seats: Vec<usize> = table
            .state
            .player_bet_states
            .iter()
            .enumerate()
            .filter(|(_seat, s)| !s.folded)
            .map(|(seat, _)| seat)
            .collect();

        if active_seats.is_empty() {
            return Err("No active players for showdown".into());
        }

        if active_seats.len() == 1 {
            // Only one player left - they win
            let winner_seat = active_seats[0];
            let pot = table.state.pot;
            self.award_pot_to_winner(table, winner_seat, pot)?;
            // Record round for all players who participated
            for player in &mut table.players {
                player.stats.record_round();
                save_player(player).map_err(|e| format!("Failed to save player: {}", e))?;
            }
            return Ok(vec![winner_seat]);
        }

        let mut best_seats = vec![active_seats[0]];
        for seat in active_seats.iter().skip(1) {
            let comparison = match table.variant {
                GameVariant::FiveCardDraw => {
                    let best_cards = self.draw_hands[&best_seats[0]].as_slice();
                    let challenger_cards = self.draw_hands[seat].as_slice();
                    compare_hands_full(best_cards, challenger_cards)
                }
                GameVariant::TexasHoldem => {
                    let mut best_cards = Vec::with_capacity(7);
                    best_cards.extend_from_slice(&self.holdem_hole_cards[&best_seats[0]]);
                    best_cards.extend_from_slice(&self.holdem_community_cards);
                    let mut challenger = Vec::with_capacity(7);
                    challenger.extend_from_slice(&self.holdem_hole_cards[seat]);
                    challenger.extend_from_slice(&self.holdem_community_cards);
                    let best_eval = evaluate_best_five_from_seven(&best_cards)?;
                    let challenger_eval = evaluate_best_five_from_seven(&challenger)?;
                    best_eval.value.cmp(&challenger_eval.value)
                }
            };
            match comparison {
                std::cmp::Ordering::Less => {
                    best_seats.clear();
                    best_seats.push(*seat);
                }
                std::cmp::Ordering::Equal => best_seats.push(*seat),
                std::cmp::Ordering::Greater => {}
            }
        }

        // Award pot to winner(s) - split if tie
        let winner_count = best_seats.len();
        let pot_per_winner = table.state.pot / winner_count as i32;
        let remainder = table.state.pot % winner_count as i32;

        for (idx, &seat) in best_seats.iter().enumerate() {
            let mut award = pot_per_winner;
            // Give remainder to first winner(s) if pot doesn't divide evenly
            if idx < remainder as usize {
                award += 1;
            }
            self.award_pot_to_winner(table, seat, award)?;
        }

        // Record round for all players who participated
        for player in &mut table.players {
            player.stats.record_round();
            save_player(player).map_err(|e| format!("Failed to save player: {}", e))?;
        }

        Ok(best_seats)
    }

    /// Award pot to a single winner. Updates balance, stats, and saves.
    fn award_pot_to_winner(
        &mut self,
        table: &mut Table,
        seat: usize,
        amount: i32,
    ) -> Result<(), String> {
        if seat >= table.players.len() {
            return Err(format!("Invalid seat index: {}", seat));
        }

        table.players[seat].balance += amount;
        table.players[seat].stats.record_pot_win();
        // Record history event
        table.players[seat]
            .history
            .push(common::accounts::history::HistoryEvent::HandResult {
                pot_won: amount,
                timestamp: Utc::now(),
                game_type: Some(table.variant.display_name().to_string()),
            });
        save_player(&table.players[seat]).map_err(|e| format!("Failed to save player: {}", e))?;

        Ok(())
    }

    /// Get a player's currently visible private cards.
    pub fn get_player_cards(&self, table: &Table, seat: usize) -> Option<Vec<Card>> {
        match table.variant {
            GameVariant::FiveCardDraw => self.draw_hands.get(&seat).map(|h| h.cards.to_vec()),
            GameVariant::TexasHoldem => self.holdem_hole_cards.get(&seat).map(|c| c.to_vec()),
        }
    }

    /// Back-compat accessor used by existing draw-focused tests.
    pub fn get_hand(&self, seat: usize) -> Option<&Hand> {
        self.draw_hands.get(&seat)
    }

    pub fn evaluate_current_player_hand(
        &self,
        table: &Table,
        seat: usize,
    ) -> Result<Option<EvaluatedHand>, String> {
        match table.variant {
            GameVariant::FiveCardDraw => {
                let hand = match self.draw_hands.get(&seat) {
                    Some(h) => h,
                    None => return Ok(None),
                };
                Ok(Some(evaluate_hand(hand.as_slice())?))
            }
            GameVariant::TexasHoldem => {
                if self.holdem_community_cards.len() < 5 {
                    return Ok(None);
                }
                let hole = match self.holdem_hole_cards.get(&seat) {
                    Some(c) => c,
                    None => return Ok(None),
                };
                let mut seven = Vec::with_capacity(7);
                seven.extend_from_slice(hole);
                seven.extend_from_slice(&self.holdem_community_cards);
                let best = evaluate_best_five_from_seven(&seven)?;
                let mut fixed = [common::poker::card::Rank::Two; 5];
                for (i, r) in best.value.tiebreak.iter().take(5).enumerate() {
                    fixed[i] = *r;
                }
                Ok(Some(EvaluatedHand {
                    rank: best.value.rank,
                    primary: fixed[0],
                    tiebreak: fixed,
                }))
            }
        }
    }

    /// Reset for next hand (cleanup).
    pub fn reset(&mut self) {
        self.draw_hands.clear();
        self.holdem_hole_cards.clear();
        self.holdem_community_cards.clear();
        self.holdem_street = None;
        self.deck = Deck::new();
        self.has_been_through_draw = false;
    }
}
