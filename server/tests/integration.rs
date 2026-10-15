//! Integration tests for complete Five-Card Draw game flow

// Integration tests - access library modules via package name
use common::accounts::player::Player;
use common::poker::variant::GameVariant;
use poker::game::GameEngine;
use poker::rules::rules::HouseRules;
use poker::table::table::Table;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;

fn make_player(name: &str, pin: i32, balance: i32) -> Player {
    Player::new(name, pin, balance)
}

/// Remove test players from the database by PIN so re-runs don't hit UNIQUE on pin.
fn cleanup_pins(pins: &[i32]) {
    let db_path = Path::new("data/accounts.db");
    if db_path.exists() {
        if let Ok(conn) = Connection::open(db_path) {
            for &pin in pins {
                let _ = conn.execute(
                    "DELETE FROM accounts WHERE pin = ?1",
                    rusqlite::params![pin],
                );
            }
        }
    }
}

/// Guard that runs cleanup on drop so DB rows are removed even when the test panics.
struct CleanupOnDrop {
    pins: Vec<i32>,
}

impl Drop for CleanupOnDrop {
    fn drop(&mut self) {
        cleanup_pins(&self.pins);
    }
}

#[test]
fn complete_hand_flow_with_stats_tracking() {
    let pins = [1001, 1002];
    cleanup_pins(&pins);
    let _cleanup = CleanupOnDrop {
        pins: pins.to_vec(),
    };

    // Create table and players
    let mut table = Table::new();
    let p1 = make_player("Alice", pins[0], 1000);
    let p2 = make_player("Bob", pins[1], 1000);
    table.add_player(p1).unwrap();
    table.add_player(p2).unwrap();

    // Create game engine
    let rules = HouseRules::default_rules();
    let mut engine = GameEngine::new(rules);

    // Track initial stats
    let alice_initial_rounds = table.players[0].stats.rounds_played;
    let bob_initial_rounds = table.players[1].stats.rounds_played;
    let alice_initial_pots = table.players[0].stats.pots_won;
    let bob_initial_pots = table.players[1].stats.pots_won;

    // Start hand
    engine.start_hand(&mut table).unwrap();
    assert_eq!(
        table.state.phase,
        poker::table::state::GamePhase::BettingRound
    );

    // First betting round: Alice bets, Bob calls
    let _current_seat = table.state.current_actor_seat;
    engine
        .process_betting_action(&mut table, poker::betting::betting::BetAction::Bet(10))
        .unwrap();

    let _current_seat = table.state.current_actor_seat;
    engine
        .process_betting_action(&mut table, poker::betting::betting::BetAction::Call)
        .unwrap();

    assert!(table.state.is_betting_round_complete());

    // Transition to draw phase
    engine.transition_to_draw(&mut table);
    assert_eq!(table.state.phase, poker::table::state::GamePhase::Draw);

    // Draw: Alice replaces 2 cards, Bob stands pat
    let mut draws = HashMap::new();
    draws.insert(0, vec![0, 1]);
    engine.process_draw_phase(&mut table, draws).unwrap();

    // Second betting round: Alice bets, Bob folds
    if table.state.phase == poker::table::state::GamePhase::BettingRound {
        let _current_seat = table.state.current_actor_seat;
        engine
            .process_betting_action(&mut table, poker::betting::betting::BetAction::Bet(5))
            .unwrap();

        let _current_seat = table.state.current_actor_seat;
        engine
            .process_betting_action(&mut table, poker::betting::betting::BetAction::Fold)
            .unwrap();
    }

    // Transition to showdown
    engine.transition_to_showdown(&mut table);
    assert_eq!(table.state.phase, poker::table::state::GamePhase::Showdown);

    // Determine winner
    let winners = engine.determine_winner_and_award_pot(&mut table).unwrap();
    assert!(!winners.is_empty());

    // Verify stats were updated
    assert_eq!(
        table.players[0].stats.rounds_played,
        alice_initial_rounds + 1
    );
    assert_eq!(table.players[1].stats.rounds_played, bob_initial_rounds + 1);
    assert_eq!(table.players[1].stats.folds, 1); // Bob folded

    // Winner should have pots_won incremented
    for &winner_seat in &winners {
        assert!(
            table.players[winner_seat].stats.pots_won
                > if winner_seat == 0 {
                    alice_initial_pots
                } else {
                    bob_initial_pots
                }
        );
    }
}

#[test]
fn persistence_across_operations() {
    // Create player and save
    let mut player = make_player("TestPlayer", 2001, 500);
    player.stats.record_round();
    player.stats.record_pot_win();

    // Save to database
    poker::persistence::storage::save_player(&player).unwrap();

    // Load and verify
    let loaded = poker::persistence::storage::load_player(player.id).unwrap();
    assert_eq!(loaded.id, player.id);
    assert_eq!(loaded.name, player.name);
    assert_eq!(loaded.balance, player.balance);
    assert_eq!(loaded.stats.rounds_played, player.stats.rounds_played);
    assert_eq!(loaded.stats.pots_won, player.stats.pots_won);

    // Clean up
    let db_path = Path::new("data/accounts.db");
    if db_path.exists() {
        let conn = Connection::open(db_path).unwrap();
        conn.execute(
            "DELETE FROM accounts WHERE id = ?1",
            rusqlite::params![player.id.to_string()],
        )
        .unwrap();
    }
}

#[test]
fn winner_determination_with_various_hands() {
    let pins = [3001, 3002];
    cleanup_pins(&pins);
    let _cleanup = CleanupOnDrop {
        pins: pins.to_vec(),
    };

    let mut table = Table::new();
    let p1 = make_player("HighCard", pins[0], 1000);
    let p2 = make_player("Pair", pins[1], 1000);
    table.add_player(p1).unwrap();
    table.add_player(p2).unwrap();

    let rules = HouseRules::default_rules();
    let mut engine = GameEngine::new(rules);

    engine.start_hand(&mut table).unwrap();

    // Simulate betting and get to showdown
    // For this test, we'll just verify the winner determination logic works
    // by manually setting up hands (in a real scenario, this would happen through gameplay)

    // Both players call to get to showdown
    let _current_seat = table.state.current_actor_seat;
    engine
        .process_betting_action(&mut table, poker::betting::betting::BetAction::Bet(1))
        .unwrap();

    let _current_seat = table.state.current_actor_seat;
    engine
        .process_betting_action(&mut table, poker::betting::betting::BetAction::Call)
        .unwrap();

    engine.transition_to_draw(&mut table);
    // No draws - both stand pat
    engine
        .process_draw_phase(&mut table, HashMap::new())
        .unwrap();

    // Second betting round - both check (bet 1, call)
    if table.state.phase == poker::table::state::GamePhase::BettingRound {
        let _current_seat = table.state.current_actor_seat;
        engine
            .process_betting_action(&mut table, poker::betting::betting::BetAction::Bet(1))
            .unwrap();

        let _current_seat = table.state.current_actor_seat;
        engine
            .process_betting_action(&mut table, poker::betting::betting::BetAction::Call)
            .unwrap();
    }

    engine.transition_to_showdown(&mut table);

    // Determine winner - should work even if hands are equal (split pot)
    let winners = engine.determine_winner_and_award_pot(&mut table);
    assert!(winners.is_ok());
    let winners = winners.unwrap();
    assert!(!winners.is_empty());
    assert!(winners.len() <= 2); // At most 2 winners (tie)
}

#[test]
fn betting_integration_with_hand_evaluation() {
    let pins = [4001, 4002];
    cleanup_pins(&pins);
    let _cleanup = CleanupOnDrop {
        pins: pins.to_vec(),
    };

    let mut table = Table::new();
    let p1 = make_player("Player1", pins[0], 1000);
    let p2 = make_player("Player2", pins[1], 1000);
    table.add_player(p1).unwrap();
    table.add_player(p2).unwrap();

    let rules = HouseRules::new(1, 100, 4, 5, 1, 2);
    let mut engine = GameEngine::new(rules.clone());

    engine.start_hand(&mut table).unwrap();

    // Verify hands were dealt
    assert!(engine.get_hand(0).is_some());
    assert!(engine.get_hand(1).is_some());

    // Betting round
    let initial_balance_0 = table.players[0].balance;
    let initial_balance_1 = table.players[1].balance;

    let current_seat = table.state.current_actor_seat;
    engine
        .process_betting_action(&mut table, poker::betting::betting::BetAction::Bet(50))
        .unwrap();

    assert_eq!(table.players[current_seat].balance, initial_balance_0 - 50);
    // Pot includes ante (2 * 5) + first bet (50)
    let expected_pot_after_bet = 2 * rules.ante_amount + 50;
    assert_eq!(table.state.pot, expected_pot_after_bet);

    let current_seat = table.state.current_actor_seat;
    engine
        .process_betting_action(&mut table, poker::betting::betting::BetAction::Call)
        .unwrap();

    assert_eq!(table.players[current_seat].balance, initial_balance_1 - 50);
    let expected_pot_after_call = expected_pot_after_bet + 50;
    assert_eq!(table.state.pot, expected_pot_after_call);

    // Complete the hand to verify winner gets the pot
    engine.transition_to_draw(&mut table);
    engine
        .process_draw_phase(&mut table, HashMap::new())
        .unwrap();

    if table.state.phase == poker::table::state::GamePhase::BettingRound {
        let _current_seat = table.state.current_actor_seat;
        engine
            .process_betting_action(&mut table, poker::betting::betting::BetAction::Bet(1))
            .unwrap();

        let _current_seat = table.state.current_actor_seat;
        engine
            .process_betting_action(&mut table, poker::betting::betting::BetAction::Call)
            .unwrap();
    }

    engine.transition_to_showdown(&mut table);
    let winners = engine.determine_winner_and_award_pot(&mut table).unwrap();

    // Winner should have received the pot
    let _total_bet = 100
        + if table.state.phase == poker::table::state::GamePhase::Showdown {
            2
        } else {
            0
        };
    for &winner_seat in &winners {
        // Winner's balance should be >= initial (they got the pot)
        // Note: exact amount depends on pot splitting if tie
        assert!(
            table.players[winner_seat].balance
                >= if winner_seat == 0 {
                    initial_balance_0
                } else {
                    initial_balance_1
                } - 50
        );
    }
}

#[test]
fn holdem_deals_streets_and_reaches_showdown() {
    let pins = [5001, 5002, 5003];
    cleanup_pins(&pins);
    let _cleanup = CleanupOnDrop {
        pins: pins.to_vec(),
    };

    let mut table = Table::new();
    table.variant = GameVariant::TexasHoldem;
    table.add_player(make_player("H1", pins[0], 1000)).unwrap();
    table.add_player(make_player("H2", pins[1], 1000)).unwrap();
    table.add_player(make_player("H3", pins[2], 1000)).unwrap();

    let rules = HouseRules::new(1, 100, 4, 0, 1, 2);
    let mut engine = GameEngine::new(rules);
    engine.start_hand(&mut table).unwrap();

    // Pre-flop: callers match the big blind until round naturally completes.
    while !table.state.is_betting_round_complete() {
        let seat = table.state.current_actor_seat;
        let need = table.state.current_bet_to_call
            - table.state.player_bet_states[seat].contribution_this_round;
        let action = if need > 0 {
            poker::betting::betting::BetAction::Call
        } else {
            poker::betting::betting::BetAction::Check
        };
        engine.process_betting_action(&mut table, action).unwrap();
    }
    engine
        .advance_holdem_after_betting_round(&mut table)
        .unwrap();
    assert_eq!(engine.holdem_community_cards().len(), 3);

    // Flop/turn/river: everyone checks through.
    for expected_board in [4usize, 5usize] {
        while !table.state.is_betting_round_complete() {
            let seat = table.state.current_actor_seat;
            let need = table.state.current_bet_to_call
                - table.state.player_bet_states[seat].contribution_this_round;
            let action = if need > 0 {
                poker::betting::betting::BetAction::Call
            } else {
                poker::betting::betting::BetAction::Check
            };
            engine.process_betting_action(&mut table, action).unwrap();
        }
        engine
            .advance_holdem_after_betting_round(&mut table)
            .unwrap();
        assert_eq!(engine.holdem_community_cards().len(), expected_board);
    }

    while !table.state.is_betting_round_complete() {
        let seat = table.state.current_actor_seat;
        let need = table.state.current_bet_to_call
            - table.state.player_bet_states[seat].contribution_this_round;
        let action = if need > 0 {
            poker::betting::betting::BetAction::Call
        } else {
            poker::betting::betting::BetAction::Check
        };
        engine.process_betting_action(&mut table, action).unwrap();
    }
    engine
        .advance_holdem_after_betting_round(&mut table)
        .unwrap();

    assert_eq!(table.state.phase, poker::table::state::GamePhase::Showdown);
    let winners = engine.determine_winner_and_award_pot(&mut table).unwrap();
    assert!(!winners.is_empty());
}
