//! Centralized tests for the poker module (deck, draw, hand evaluation).

#[cfg(test)]
mod deck {
    use crate::poker::card::Card;
    use crate::poker::deck::Deck;
    use std::collections::HashSet;

    #[test]
    fn new_deck_has_52_unique_cards() {
        let d = Deck::new();
        assert_eq!(d.len(), 52);

        let mut deck = Deck::new();
        let set: HashSet<Card> = (0..52).filter_map(|_| deck.deal_one()).collect();
        assert_eq!(set.len(), 52);
    }

    #[test]
    fn dealing_reduces_deck_size_and_no_duplicates() {
        let mut d = Deck::new();
        d.shuffle();

        let mut seen: HashSet<Card> = HashSet::new();
        for _ in 0..52 {
            let c: Card = d.deal_one().expect("should have card");
            assert!(seen.insert(c), "duplicate card dealt");
        }
        assert!(d.deal_one().is_none());
        assert!(d.is_empty());
    }
}

#[cfg(test)]
mod draw {
    use crate::poker::card::Card;
    use crate::poker::deck::Deck;
    use crate::poker::draw::{deal_hand, deal_hands_round_robin, replace_cards};
    use crate::poker::hand::Hand;
    use std::collections::HashSet;

    #[test]
    fn deal_hand_returns_five_cards_and_consumes_deck() {
        let mut deck = Deck::new();
        let len_before = deck.len();
        let hand = deal_hand(&mut deck).expect("full deck should deal a hand");
        assert_eq!(hand.cards.len(), 5);
        assert_eq!(deck.len(), len_before - 5);
    }

    #[test]
    fn deal_hand_returns_none_when_insufficient_cards() {
        let mut deck = Deck::new();
        for _ in 0..52 - 4 {
            deck.deal_one();
        }
        assert_eq!(deck.len(), 4);
        assert!(deal_hand(&mut deck).is_none());
    }

    #[test]
    fn deal_hand_deals_unique_cards() {
        let mut deck = Deck::new();
        deck.shuffle();
        let hand = deal_hand(&mut deck).unwrap();
        let set: HashSet<Card> = hand.cards.iter().copied().collect();
        assert_eq!(set.len(), 5, "hand should have 5 unique cards");
    }

    #[test]
    fn deal_hands_round_robin_returns_none_for_zero_players() {
        let mut deck = Deck::new();
        assert!(deal_hands_round_robin(&mut deck, 0).is_none());
    }

    #[test]
    fn deal_hands_round_robin_returns_none_when_insufficient_cards() {
        let mut deck = Deck::new();
        for _ in 0..52 - 9 {
            deck.deal_one();
        }
        assert!(deal_hands_round_robin(&mut deck, 2).is_none());
    }

    #[test]
    fn deal_hands_round_robin_correct_count_and_sizes() {
        let mut deck = Deck::new();
        deck.shuffle();
        let hands = deal_hands_round_robin(&mut deck, 3).unwrap();
        assert_eq!(hands.len(), 3);
        for hand in &hands {
            assert_eq!(hand.cards.len(), 5);
        }
        assert_eq!(deck.len(), 52 - 15);
    }

    #[test]
    fn deal_hands_round_robin_all_cards_unique_across_hands() {
        let mut deck = Deck::new();
        deck.shuffle();
        let hands = deal_hands_round_robin(&mut deck, 2).unwrap();
        let all_cards: Vec<Card> = hands.iter().flat_map(|h| h.cards.iter().copied()).collect();
        let set: HashSet<Card> = all_cards.into_iter().collect();
        assert_eq!(set.len(), 10, "all 10 cards across hands should be unique");
    }

    #[test]
    fn deal_hands_round_robin_deals_one_per_player_per_round() {
        let mut deck = Deck::new();
        deck.shuffle();
        let mut deck_copy = deck.clone();
        let hands = deal_hands_round_robin(&mut deck, 2).unwrap();

        let mut p0: Vec<Card> = Vec::new();
        let mut p1: Vec<Card> = Vec::new();
        for _ in 0..5 {
            p0.push(deck_copy.deal_one().unwrap());
            p1.push(deck_copy.deal_one().unwrap());
        }
        let expected_p0 = Hand::new(p0.try_into().unwrap());
        let expected_p1 = Hand::new(p1.try_into().unwrap());

        assert_eq!(hands[0], expected_p0);
        assert_eq!(hands[1], expected_p1);
    }

    #[test]
    fn replace_cards_replaces_selected_positions() {
        let mut deck = Deck::new();
        let hand = deal_hand(&mut deck).unwrap();
        let original_cards = hand.cards;
        let to_replace = [0usize, 2];
        let new_hand = replace_cards(&mut deck, &hand, &to_replace).unwrap();

        assert_eq!(new_hand.cards[1], original_cards[1]);
        assert_eq!(new_hand.cards[3], original_cards[3]);
        assert_eq!(new_hand.cards[4], original_cards[4]);
        assert_ne!(new_hand.cards[0], original_cards[0]);
        assert_ne!(new_hand.cards[2], original_cards[2]);
    }

    #[test]
    fn replace_cards_empty_indices_returns_unchanged_hand() {
        let mut deck = Deck::new();
        let hand = deal_hand(&mut deck).unwrap();
        let new_hand = replace_cards(&mut deck, &hand, &[]).unwrap();
        assert_eq!(new_hand.cards, hand.cards);
    }

    #[test]
    fn replace_cards_rejects_invalid_index() {
        let mut deck = Deck::new();
        let hand = deal_hand(&mut deck).unwrap();
        assert!(replace_cards(&mut deck, &hand, &[5]).is_none());
        assert!(replace_cards(&mut deck, &hand, &[0, 10]).is_none());
    }

    #[test]
    fn replace_cards_rejects_duplicate_indices() {
        let mut deck = Deck::new();
        let hand = deal_hand(&mut deck).unwrap();
        assert!(replace_cards(&mut deck, &hand, &[0, 0]).is_none());
        assert!(replace_cards(&mut deck, &hand, &[1, 2, 1]).is_none());
    }

    #[test]
    fn replace_cards_returns_none_when_deck_runs_out() {
        let mut deck = Deck::new();
        let hand = deal_hand(&mut deck).unwrap();
        for _ in 0..52 - 5 {
            deck.deal_one();
        }
        assert!(deck.len() < 3);
        assert!(replace_cards(&mut deck, &hand, &[0, 1, 2]).is_none());
    }
}

#[cfg(test)]
mod hand_eval {
    use crate::poker::card::{Card, Rank, Suit};
    use crate::poker::hand_eval::{
        HandRank, compare_hands_first_level, compare_hands_full, evaluate_best_five_from_seven,
        evaluate_hand,
    };
    use std::cmp::Ordering;

    fn c(s: Suit, r: Rank) -> Card {
        Card { suit: s, rank: r }
    }

    #[test]
    fn ranks_royal_flush() {
        let hand = [
            c(Suit::Spades, Rank::Ten),
            c(Suit::Spades, Rank::Jack),
            c(Suit::Spades, Rank::Queen),
            c(Suit::Spades, Rank::King),
            c(Suit::Spades, Rank::Ace),
        ];
        let e = evaluate_hand(&hand).unwrap();
        assert_eq!(e.rank, HandRank::RoyalFlush);
        assert_eq!(e.primary, Rank::Ace);
    }

    #[test]
    fn ranks_straight_flush() {
        let hand = [
            c(Suit::Hearts, Rank::Nine),
            c(Suit::Hearts, Rank::Ten),
            c(Suit::Hearts, Rank::Jack),
            c(Suit::Hearts, Rank::Queen),
            c(Suit::Hearts, Rank::King),
        ];
        let e = evaluate_hand(&hand).unwrap();
        assert_eq!(e.rank, HandRank::StraightFlush);
        assert_eq!(e.primary, Rank::King);
    }

    #[test]
    fn ranks_full_house() {
        let hand = [
            c(Suit::Clubs, Rank::Nine),
            c(Suit::Hearts, Rank::Nine),
            c(Suit::Spades, Rank::Nine),
            c(Suit::Diamonds, Rank::King),
            c(Suit::Clubs, Rank::King),
        ];
        let e = evaluate_hand(&hand).unwrap();
        assert_eq!(e.rank, HandRank::FullHouse);
        assert_eq!(e.primary, Rank::Nine);
    }

    #[test]
    fn wheel_straight_ace_low() {
        let hand = [
            c(Suit::Clubs, Rank::Ace),
            c(Suit::Hearts, Rank::Two),
            c(Suit::Spades, Rank::Three),
            c(Suit::Diamonds, Rank::Four),
            c(Suit::Clubs, Rank::Five),
        ];
        let e = evaluate_hand(&hand).unwrap();
        assert_eq!(e.rank, HandRank::Straight);
        assert_eq!(e.primary, Rank::Five);
    }

    #[test]
    fn royal_flush_beats_straight_flush() {
        let royal = [
            c(Suit::Spades, Rank::Ten),
            c(Suit::Spades, Rank::Jack),
            c(Suit::Spades, Rank::Queen),
            c(Suit::Spades, Rank::King),
            c(Suit::Spades, Rank::Ace),
        ];

        let straight_flush = [
            c(Suit::Hearts, Rank::Nine),
            c(Suit::Hearts, Rank::Ten),
            c(Suit::Hearts, Rank::Jack),
            c(Suit::Hearts, Rank::Queen),
            c(Suit::Hearts, Rank::King),
        ];

        let ord = compare_hands_first_level(&royal, &straight_flush).unwrap();
        assert_eq!(ord, Ordering::Greater);
    }

    #[test]
    fn first_level_tie_break_pair_rank_only() {
        let a = [
            c(Suit::Clubs, Rank::Jack),
            c(Suit::Hearts, Rank::Jack),
            c(Suit::Spades, Rank::Two),
            c(Suit::Diamonds, Rank::Three),
            c(Suit::Clubs, Rank::Ace),
        ];
        let b = [
            c(Suit::Diamonds, Rank::Jack),
            c(Suit::Spades, Rank::Jack),
            c(Suit::Hearts, Rank::Four),
            c(Suit::Clubs, Rank::Five),
            c(Suit::Spades, Rank::King),
        ];

        let ord = compare_hands_first_level(&a, &b).unwrap();
        assert_eq!(ord, Ordering::Equal);
    }

    #[test]
    fn first_level_tie_break_two_pair_high_pair_only() {
        let a = [
            c(Suit::Clubs, Rank::King),
            c(Suit::Hearts, Rank::King),
            c(Suit::Spades, Rank::Two),
            c(Suit::Diamonds, Rank::Two),
            c(Suit::Clubs, Rank::Three),
        ];
        let b = [
            c(Suit::Clubs, Rank::Ace),
            c(Suit::Hearts, Rank::Ace),
            c(Suit::Spades, Rank::Queen),
            c(Suit::Diamonds, Rank::Queen),
            c(Suit::Clubs, Rank::Four),
        ];

        let ord = compare_hands_first_level(&a, &b).unwrap();
        assert_eq!(ord, Ordering::Less);
    }

    #[test]
    fn full_tie_break_uses_kicker_for_pairs() {
        let a = [
            c(Suit::Clubs, Rank::Ace),
            c(Suit::Hearts, Rank::Ace),
            c(Suit::Spades, Rank::King),
            c(Suit::Diamonds, Rank::Ten),
            c(Suit::Clubs, Rank::Two),
        ];
        let b = [
            c(Suit::Diamonds, Rank::Ace),
            c(Suit::Spades, Rank::Ace),
            c(Suit::Hearts, Rank::Queen),
            c(Suit::Clubs, Rank::Ten),
            c(Suit::Spades, Rank::Two),
        ];
        assert_eq!(compare_hands_full(&a, &b), Ordering::Greater);
    }

    #[test]
    fn holdem_best_five_from_seven_selects_straight() {
        let seven = [
            c(Suit::Clubs, Rank::Ace),
            c(Suit::Hearts, Rank::King),
            c(Suit::Spades, Rank::Queen),
            c(Suit::Diamonds, Rank::Jack),
            c(Suit::Clubs, Rank::Ten),
            c(Suit::Spades, Rank::Two),
            c(Suit::Hearts, Rank::Three),
        ];
        let best = evaluate_best_five_from_seven(&seven).unwrap();
        assert_eq!(best.value.rank, HandRank::Straight);
        assert_eq!(best.value.tiebreak[0], Rank::Ace);
    }
}
