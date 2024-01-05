use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::{self, Database, OnBattlefield},
    in_play::{CardId, InGraveyard},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::Stack,
    turns::Turn,
};

#[test]
fn opponent_artifact_etb_tappd() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .pretty()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ENTER)
        .with_writer(std::io::stderr)
        .try_init();

    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player1 = all_players.new_player(String::default(), 20);
    let player2 = all_players.new_player(String::default(), 20);
    let turn = Turn::new(&mut db, &all_players);

    let card = CardId::upload(&mut db, &cards, player1, "Dauntless Dismantler");
    let card2 = CardId::upload(&mut db, &cards, player2, "Abzan Banner");

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(card2.tapped(&db));

    Ok(())
}

#[test]
fn opponent_artifact_destroys_artifacts() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .pretty()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ENTER)
        .with_writer(std::io::stderr)
        .try_init();

    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player1 = all_players.new_player(String::default(), 20);
    all_players[player1].infinite_mana();
    let player2 = all_players.new_player(String::default(), 20);

    let turn = Turn::new(&mut db, &all_players);

    let card = CardId::upload(&mut db, &cards, player1, "Dauntless Dismantler");
    let card2 = CardId::upload(&mut db, &cards, player2, "Abzan Banner");

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), [card, card2]);

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, &None, player1, card, 0);
    // Pay white
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay 3x2 X mana
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Done paying X
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), []);
    assert_eq!(player1.get_cards::<InGraveyard>(&mut db), [card]);
    assert_eq!(player2.get_cards::<InGraveyard>(&mut db), [card2]);

    Ok(())
}
