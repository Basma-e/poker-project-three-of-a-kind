//! Hand ranking engine for 5-card poker hands (Phase 1).

use std::cmp::Ordering;
use std::collections::HashMap;

use super::card::{Card, Rank};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandRank {
    HighCard = 1,
    OnePair = 2,
    TwoPair = 3,
    ThreeOfAKind = 4,
    Straight = 5,
    Flush = 6,
    FullHouse = 7,
    FourOfAKind = 8,
    StraightFlush = 9,
    RoyalFlush = 10,
}

/// Result of evaluating a hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluatedHand {
    pub rank: HandRank,
    pub primary: Rank,
    /// Full tie-break vector ordered from most-significant to least-significant.
    pub tiebreak: [Rank; 5],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HandValue {
    pub rank: HandRank,
    pub tiebreak: Vec<Rank>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestFiveFromSeven {
    pub five_cards: Vec<Card>,
    pub value: HandValue,
}

/// Public: evaluate a 5-card hand.
/// - Returns the category (HandRank)
pub fn evaluate_hand(cards: &[Card]) -> Result<EvaluatedHand, String> {
    if cards.len() != 5 {
        return Err("Hand must contain exactly 5 cards".into());
    }

    // collect ranks
    let mut ranks: Vec<Rank> = cards.iter().map(|c| c.rank).collect();
    ranks.sort(); // ascending by Rank

    let is_flush = cards.iter().all(|c| c.suit == cards[0].suit);
    let (is_straight, straight_high) = straight_info(&ranks);

    // count ranks
    let mut counts: HashMap<Rank, usize> = HashMap::new();
    for r in ranks.iter().copied() {
        *counts.entry(r).or_insert(0) += 1;
    }

    // convert to list of (count, rank) for pattern detection
    let mut count_list: Vec<(usize, Rank)> = counts.into_iter().map(|(r, c)| (c, r)).collect();
    // sort by count descending then rank descending (useful for primary extraction)
    count_list.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

    let build = |rank: HandRank, tiebreak: Vec<Rank>| -> EvaluatedHand {
        let mut fixed = [Rank::Two; 5];
        for (i, r) in tiebreak.iter().take(5).enumerate() {
            fixed[i] = *r;
        }
        EvaluatedHand {
            rank,
            primary: fixed[0],
            tiebreak: fixed,
        }
    };

    // Straight flush (including Royal Flush)
    if is_straight && is_flush {
        if is_royal_ranks(&ranks) {
            return Ok(build(HandRank::RoyalFlush, vec![Rank::Ace]));
        }

        return Ok(build(HandRank::StraightFlush, vec![straight_high]));
    }

    // Four of a kind
    if count_list[0].0 == 4 {
        let quad = count_list[0].1;
        let kicker = count_list[1].1;
        return Ok(build(HandRank::FourOfAKind, vec![quad, kicker]));
    }

    // Full house
    if count_list[0].0 == 3 && count_list[1].0 == 2 {
        return Ok(build(
            HandRank::FullHouse,
            vec![count_list[0].1, count_list[1].1],
        ));
    }

    // Flush
    if is_flush {
        let mut desc = ranks.clone();
        desc.sort_by(|a, b| b.cmp(a));
        return Ok(build(HandRank::Flush, desc));
    }

    // Straight
    if is_straight {
        return Ok(build(HandRank::Straight, vec![straight_high]));
    }

    // Three of a kind
    if count_list[0].0 == 3 {
        let trip = count_list[0].1;
        let mut kickers = vec![count_list[1].1, count_list[2].1];
        kickers.sort_by(|a, b| b.cmp(a));
        let mut tiebreak = vec![trip];
        tiebreak.extend(kickers);
        return Ok(build(HandRank::ThreeOfAKind, tiebreak));
    }

    // Two pair
    if count_list[0].0 == 2 && count_list[1].0 == 2 {
        let mut pairs = vec![count_list[0].1, count_list[1].1];
        pairs.sort_by(|a, b| b.cmp(a));
        let kicker = count_list[2].1;
        let mut tiebreak = pairs;
        tiebreak.push(kicker);
        return Ok(build(HandRank::TwoPair, tiebreak));
    }

    // One pair
    if count_list[0].0 == 2 {
        let pair = count_list[0].1;
        let mut kickers = vec![count_list[1].1, count_list[2].1, count_list[3].1];
        kickers.sort_by(|a, b| b.cmp(a));
        let mut tiebreak = vec![pair];
        tiebreak.extend(kickers);
        return Ok(build(HandRank::OnePair, tiebreak));
    }

    // High card
    let mut desc = ranks;
    desc.sort_by(|a, b| b.cmp(a));
    Ok(build(HandRank::HighCard, desc))
}

/// Compare two 5-card hands using:
/// 1) Hand category (HandRank)
/// 2) First-level tie breaker only (primary)
/// If still tied, return Equal (caller should split pot).
pub fn compare_hands_first_level(a: &[Card], b: &[Card]) -> Result<Ordering, String> {
    let ea = evaluate_hand(a)?;
    let eb = evaluate_hand(b)?;

    let by_rank = ea.rank.cmp(&eb.rank);
    if by_rank != Ordering::Equal {
        return Ok(by_rank);
    }

    let by_primary = ea.primary.cmp(&eb.primary);
    Ok(by_primary)
}

/// Compare two 5-card hands with full tiebreak rules.
pub fn compare_hands_full(a: &[Card], b: &[Card]) -> Ordering {
    let ea = evaluate_hand(a).expect("valid 5-card hand expected");
    let eb = evaluate_hand(b).expect("valid 5-card hand expected");
    match ea.rank.cmp(&eb.rank) {
        Ordering::Equal => ea.tiebreak.cmp(&eb.tiebreak),
        ord => ord,
    }
}

pub fn evaluate_best_five_from_seven(cards: &[Card]) -> Result<BestFiveFromSeven, String> {
    if cards.len() != 7 {
        return Err("Need exactly 7 cards to evaluate Hold'em hand".into());
    }
    let mut best: Option<BestFiveFromSeven> = None;
    for i in 0..7 {
        for j in (i + 1)..7 {
            for k in (j + 1)..7 {
                for l in (k + 1)..7 {
                    for m in (l + 1)..7 {
                        let combo = vec![cards[i], cards[j], cards[k], cards[l], cards[m]];
                        let eval = evaluate_hand(&combo)?;
                        let value = HandValue {
                            rank: eval.rank,
                            tiebreak: eval.tiebreak.to_vec(),
                        };
                        let candidate = BestFiveFromSeven {
                            five_cards: combo,
                            value,
                        };
                        if best
                            .as_ref()
                            .map(|b| candidate.value > b.value)
                            .unwrap_or(true)
                        {
                            best = Some(candidate);
                        }
                    }
                }
            }
        }
    }
    best.ok_or("Could not evaluate best five-card hand".into())
}

/// Detect straight and return (is_straight, high_card_of_straight).
/// Handles wheel straight A-2-3-4-5 (high = Five).
fn straight_info(sorted_ranks: &[Rank]) -> (bool, Rank) {
    // Special case: A-2-3-4-5
    let wheel = [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Ace];
    if sorted_ranks == wheel {
        return (true, Rank::Five);
    }

    // General: consecutive values
    for i in 0..4 {
        let cur = sorted_ranks[i].value();
        let nxt = sorted_ranks[i + 1].value();
        if nxt != cur + 1 {
            return (false, *sorted_ranks.last().unwrap());
        }
    }
    (true, *sorted_ranks.last().unwrap())
}

/// Detect royal straight (10 to Ace) to help detect royal flush.
fn is_royal_ranks(sorted_ranks: &[Rank]) -> bool {
    sorted_ranks == [Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]
}
