use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::{CardId, Database},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    types::{Subtype, Type},
};

#[test]
fn metamorphosis() -> anyhow::Result<()> {
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
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let mut db = Database::new(all_players);

    let mantle = CardId::upload(&mut db, &cards, player, "Paradise Mantle");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, mantle, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let majestic = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");

    let mut results = majestic.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: mantle }]],
        None,
        vec![],
    );
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(mantle.power(&db), Some(4));
    assert_eq!(mantle.toughness(&db), Some(4));
    assert_eq!(
        db[mantle].modified_subtypes,
        IndexSet::from([Subtype::Equipment, Subtype::Angel])
    );
    assert_eq!(
        db[mantle].modified_types,
        IndexSet::from([Type::Artifact, Type::Creature])
    );
    assert!(mantle.flying(&db));

    Ok(())
}

#[test]
fn metamorphosis_bear() -> anyhow::Result<()> {
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
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let mut db = Database::new(all_players);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, bear, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let majestic = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");

    let mut results = majestic.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear }]],
        None,
        vec![],
    );
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(bear.power(&db), Some(4));
    assert_eq!(bear.toughness(&db), Some(4));
    assert_eq!(
        db[bear].modified_subtypes,
        IndexSet::from([Subtype::Bear, Subtype::Angel])
    );
    assert_eq!(
        db[bear].modified_types,
        IndexSet::from([Type::Artifact, Type::Creature])
    );
    assert!(bear.flying(&db));

    Ok(())
}
