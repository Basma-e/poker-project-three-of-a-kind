//! Integration test: table + betting + rules without deck/hand/persistence.
//! Run with: cargo test -p poker

use common::accounts::player::Player;
use poker::betting::betting::{BetAction, BettingRound};
use poker::rules::rules::HouseRules;
use poker::table::state::GamePhase;
use poker::table::table::Table;

fn make_player(name: &str, balance: i32) -> Player {
    Player::new(name, 0, balance)
}

#[test]
fn full_betting_round_two_players() {
    // 1. Create table and add 2 players (US 01.01.01)
    let mut table = Table::new();
    assert!(!table.can_start());
    table.add_player(make_player("Alice", 100)).unwrap();
    table.add_player(make_player("Bob", 100)).unwrap();
    assert!(table.can_start());

    // 2. Start a betting round (dealer seat 0, first to act is seat 1)
    let rules = HouseRules::default_rules();
    table.state.start_betting_round(table.players.len(), 0);
    assert_eq!(table.state.phase, GamePhase::BettingRound);
    assert_eq!(table.state.current_actor_seat, 1);

    // 3. Bet, call, then round is complete (US 01.01.03)
    let round = BettingRound::new(rules.clone());
    round
        .apply_action(&mut table.state, &mut table.players, 1, BetAction::Bet(10))
        .unwrap();
    assert_eq!(table.state.pot, 10);
    assert_eq!(table.players[1].balance, 90);

    round
        .apply_action(&mut table.state, &mut table.players, 0, BetAction::Call)
        .unwrap();
    assert_eq!(table.state.pot, 20);
    assert_eq!(table.players[0].balance, 90);
    assert!(table.state.is_betting_round_complete());
}

#[test]
fn full_betting_round_with_raise() {
    let mut table = Table::new();
    table.add_player(make_player("A", 200)).unwrap();
    table.add_player(make_player("B", 200)).unwrap();
    let rules = HouseRules::new(5, 100, 4, 5, 1, 2);
    table.state.start_betting_round(2, 0);
    let round = BettingRound::new(rules);

    round
        .apply_action(&mut table.state, &mut table.players, 1, BetAction::Bet(10))
        .unwrap();
    round
        .apply_action(
            &mut table.state,
            &mut table.players,
            0,
            BetAction::Raise(25),
        )
        .unwrap();
    assert_eq!(table.state.current_bet_to_call, 25);
    assert_eq!(table.state.pot, 10 + 25);
    round
        .apply_action(&mut table.state, &mut table.players, 1, BetAction::Call)
        .unwrap();
    assert_eq!(table.state.pot, 10 + 25 + 15);
    assert!(table.state.is_betting_round_complete());
}
