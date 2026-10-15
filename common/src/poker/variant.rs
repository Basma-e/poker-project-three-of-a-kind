//! Supported poker game variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameVariant {
    FiveCardDraw,
    TexasHoldem,
}

impl GameVariant {
    pub fn from_token(token: &str) -> Option<Self> {
        match token.to_ascii_uppercase().as_str() {
            "DRAW" | "FIVE_CARD_DRAW" | "FIVECARDDRAW" => Some(Self::FiveCardDraw),
            "HOLD" | "HOLD_EM" | "HOLDEM" | "TEXAS_HOLDEM" | "TEXAS_HOLD_EM" => {
                Some(Self::TexasHoldem)
            }
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::FiveCardDraw => "FiveCardDraw",
            Self::TexasHoldem => "TexasHoldem",
        }
    }
}
