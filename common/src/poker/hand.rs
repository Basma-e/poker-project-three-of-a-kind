//! Hand model: exactly 5 cards for Five-Card Draw (Phase 1)

use super::card::Card;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hand {
    pub cards: [Card; 5],
}

impl Hand {
    pub fn new(cards: [Card; 5]) -> Self {
        Self { cards }
    }

    pub fn as_slice(&self) -> &[Card] {
        &self.cards
    }
}
