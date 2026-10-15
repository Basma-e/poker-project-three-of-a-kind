/// House rules for betting (US 01.01.04): min/max bet, raise limits.

// Struct storing all configurable betting rules
#[derive(Clone)]
pub struct HouseRules {
    pub min_bet: i32,              // Minimum allowed bet/raise
    pub max_bet: i32,              // Maximum allowed bet/raise
    pub max_raises_per_round: u32, // Max raises allowed per betting round
    pub ante_amount: i32,          // Ante amount required from each player before dealing
    pub small_blind: i32,          // Small blind for Hold'em
    pub big_blind: i32,            // Big blind for Hold'em
}

// Implementation block for house rule logic
impl HouseRules {
    /// Create house rules with min/max bet, max raises per round, and ante amount.
    pub fn new(
        min_bet: i32,
        max_bet: i32,
        max_raises_per_round: u32,
        ante_amount: i32,
        small_blind: i32,
        big_blind: i32,
    ) -> Self {
        Self {
            min_bet,
            max_bet,
            max_raises_per_round,
            ante_amount,
            small_blind,
            big_blind,
        }
    }

    /// Default rules: min 1, max 100, 4 raises per round, no ante, blinds 5/10.
    pub fn default_rules() -> Self {
        Self::new(1, 100, 4, 0, 5, 10)
    }

    /// Validate a single bet/raise amount against min/max (US 01.01.04).
    pub fn validate_bet(&self, amount: i32) -> bool {
        amount >= self.min_bet && amount <= self.max_bet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_bet_accepts_within_limits() {
        let rules = HouseRules::new(5, 50, 3, 5, 1, 2);
        assert!(rules.validate_bet(5));
        assert!(rules.validate_bet(50));
        assert!(rules.validate_bet(25));
    }

    #[test]
    fn validate_bet_rejects_below_min() {
        let rules = HouseRules::new(5, 50, 3, 5, 1, 2);
        assert!(!rules.validate_bet(4));
        assert!(!rules.validate_bet(0));
    }

    #[test]
    fn validate_bet_rejects_above_max() {
        let rules = HouseRules::new(5, 50, 3, 5, 1, 2);
        assert!(!rules.validate_bet(51));
        assert!(!rules.validate_bet(100));
    }

    #[test]
    fn default_rules() {
        let r = HouseRules::default_rules();
        assert_eq!(r.min_bet, 1);
        assert_eq!(r.max_bet, 100);
        assert_eq!(r.max_raises_per_round, 4);
        assert_eq!(r.ante_amount, 0);
        assert_eq!(r.small_blind, 5);
        assert_eq!(r.big_blind, 10);
    }
}
