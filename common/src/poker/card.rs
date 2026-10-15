//! Card model for a standard 52-card deck (no jokers).

use std::fmt;

/// The four suits in a standard deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

/// Rank values from Two (2) to Ace (14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rank {
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
    Ace = 14,
}

impl Rank {
    /// Convert to numeric value (2..14).
    pub fn value(self) -> u8 {
        self as u8
    }

    /// All ranks in ascending order.
    pub fn all() -> [Rank; 13] {
        use Rank::*;
        [
            Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King, Ace,
        ]
    }
}

/// A playing card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self { suit, rank }
    }
}

/// Define display print out for card (e.g., A♠, T♥, etc.).
impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Rank::*;
        use Suit::*;

        let r = match self.rank {
            Two => "2",
            Three => "3",
            Four => "4",
            Five => "5",
            Six => "6",
            Seven => "7",
            Eight => "8",
            Nine => "9",
            Ten => "10",
            Jack => "J",
            Queen => "Q",
            King => "K",
            Ace => "A",
        };

        let s = match self.suit {
            Hearts => "♥",
            Diamonds => "♦",
            Clubs => "♣",
            Spades => "♠",
        };

        write!(f, "{}{}", r, s)
    }
}
