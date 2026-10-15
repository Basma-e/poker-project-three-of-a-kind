//! Deck implementation (standard 52 cards).

use rand::seq::SliceRandom;
use rand::thread_rng;

use super::card::{Card, Rank, Suit};

/// A deck of cards (top of deck is the end of the vector).
#[derive(Debug, Clone)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    /// TODO (Person B, US 01.01.06):
    /// Create a standard 52-card deck
    pub fn new() -> Self {
        let mut cards = Vec::with_capacity(52);
        for suit in [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
            for rank in Rank::all() {
                cards.push(Card { suit, rank });
            }
        }
        Self { cards }
    }

    /// Shuffle the deck in-place.
    pub fn shuffle(&mut self) {
        let mut rng = thread_rng();
        self.cards.shuffle(&mut rng);
    }

    /// Remaining number of cards.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Deal one card from the top of the deck.
    pub fn deal_one(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    /// Deal `n` cards, returning them in dealt order.
    pub fn deal_n(&mut self, n: usize) -> Vec<Card> {
        (0..n).filter_map(|_| self.deal_one()).collect()
    }
}
