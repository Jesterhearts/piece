use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    turns::Turn,
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
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let turn = Turn::new(&mut db, &all_players);

    let mantle = CardId::upload(&mut db, &cards, player, "Paradise Mantle");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, mantle, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let majestic = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");

    majestic.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: mantle }]],
        None,
        vec![],
    );

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(mantle.power(&mut db), Some(4));
    assert_eq!(mantle.toughness(&mut db), Some(4));
    assert_eq!(
        mantle.subtypes(&db),
        IndexSet::from([Subtype::Equipment, Subtype::Angel])
    );
    assert_eq!(
        mantle.types(&db),
        IndexSet::from([Type::Artifact, Type::Creature])
    );
    assert!(mantle.flying(&mut db));

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
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let turn = Turn::new(&mut db, &all_players);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, bear, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let majestic = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");

    majestic.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear }]],
        None,
        vec![],
    );

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(bear.power(&mut db), Some(4));
    assert_eq!(bear.toughness(&mut db), Some(4));
    assert_eq!(
        bear.subtypes(&db),
        IndexSet::from([Subtype::Bear, Subtype::Angel])
    );
    assert_eq!(
        bear.types(&db),
        IndexSet::from([Type::Artifact, Type::Creature])
    );
    assert!(bear.flying(&mut db));

    Ok(())
}
