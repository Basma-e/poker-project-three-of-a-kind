//! Five-Card Draw dealing helpers (Phase 1).

use super::card::Card;
use super::deck::Deck;
use super::hand::Hand;

/// TODO (Person B, US 01.01.02):
/// Deal a 5-card hand from the deck.
/// Returns None if the deck does not have enough cards.
pub fn deal_hand(deck: &mut Deck) -> Option<Hand> {
    if deck.len() < 5 {
        return None;
    }

    let dealt = deck.deal_n(5);
    Some(Hand::new([
        dealt[0], dealt[1], dealt[2], dealt[3], dealt[4],
    ]))
}

/// Deal hands in round-robin order: one card to each player, repeated until each has 5 cards.
/// Returns None if the deck does not have enough cards (need 5 * player_count).
pub fn deal_hands_round_robin(deck: &mut Deck, player_count: usize) -> Option<Vec<Hand>> {
    if player_count == 0 || deck.len() < 5 * player_count {
        return None;
    }

    let mut player_cards: Vec<Vec<Card>> =
        (0..player_count).map(|_| Vec::with_capacity(5)).collect();

    for _ in 0..5 {
        for seat in 0..player_count {
            let card = deck.deal_one()?;
            player_cards[seat].push(card);
        }
    }

    let hands: Vec<Hand> = player_cards
        .into_iter()
        .map(|cards| {
            let arr: [Card; 5] = cards.try_into().expect("each player has exactly 5 cards");
            Hand::new(arr)
        })
        .collect();

    Some(hands)
}

/// Replace selected card positions (0..4) with new cards from the deck.
/// Returns None if deck doesn't have enough cards or indices are invalid/duplicated.
pub fn replace_cards(deck: &mut Deck, hand: &Hand, replace_idxs: &[usize]) -> Option<Hand> {
    // validate indices: 0..4 and unique
    let mut seen = [false; 5];
    for &i in replace_idxs {
        if i >= 5 || seen[i] {
            return None;
        }
        seen[i] = true;
    }

    let mut new_cards = hand.cards; // copy array
    for &i in replace_idxs {
        new_cards[i] = deck.deal_one()?;
    }

    Some(Hand::new(new_cards))
}
