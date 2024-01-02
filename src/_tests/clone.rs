use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::CardId,
    in_play::{self, Database, InGraveyard},
    load_cards,
    player::AllPlayers,
    turns::Turn,
};

#[test]
fn etb_clones() -> anyhow::Result<()> {
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
    let player = all_players.new_player("Player".to_string(), 20);
    let turn = Turn::new(&mut db, &all_players);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, creature, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let clone = CardId::upload(&mut db, &cards, player, "Clone");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, clone, None);

    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(clone.cloning(&db), Some(creature.into()));

    Ok(())
}

#[test]
fn etb_no_targets_dies() -> anyhow::Result<()> {
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
    let player = all_players.new_player("Player".to_string(), 20);
    let turn = Turn::new(&mut db, &all_players);

    let clone = CardId::upload(&mut db, &cards, player, "Clone");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, clone, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::check_sba(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(in_play::cards::<InGraveyard>(&mut db), [clone]);

    Ok(())
}
