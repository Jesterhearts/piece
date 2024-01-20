use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    in_play::Database,
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    protogen::{
        ids::CardId,
        types::{Subtype, Type},
    },
    stack::Stack,
    types::{SubtypeSet, TypeSet},
};

#[test]
fn x_is_zero() -> anyhow::Result<()> {
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

    let player = all_players.new_player("player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let card = CardId::upload(&mut db, &cards, player, "Abuelo's Awakening");
    let target = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let non_target = CardId::upload(&mut db, &cards, player, "Abzan Runemark");

    target.move_to_graveyard(&mut db);
    non_target.move_to_graveyard(&mut db);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card, true);
    // Choose the target
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay the white
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay the generic
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Skip the X
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let on_battlefield = db
        .battlefield
        .battlefields
        .values()
        .flat_map(|b| b.iter())
        .cloned()
        .collect_vec();
    assert_eq!(on_battlefield, [target.clone()]);
    assert_eq!(target.power(&db), Some(1));
    assert_eq!(target.toughness(&db), Some(1));
    assert_eq!(
        db[&target].modified_types,
        TypeSet::from([Type::CREATURE, Type::ARTIFACT])
    );
    assert_eq!(
        db[&target].modified_subtypes,
        SubtypeSet::from([Subtype::SPIRIT])
    );

    Ok(())
}

#[test]
fn x_is_two() -> anyhow::Result<()> {
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
    let player = all_players.new_player("player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let card = CardId::upload(&mut db, &cards, player, "Abuelo's Awakening");
    let target = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let non_target = CardId::upload(&mut db, &cards, player, "Abzan Runemark");

    target.move_to_graveyard(&mut db);
    non_target.move_to_graveyard(&mut db);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card, true);
    // Choose the target
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay the white
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay the generic
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // pay 1 for X
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    // pay 1 for X
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Skip paying x
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Add card to stack & pay
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let on_battlefield = db
        .battlefield
        .battlefields
        .values()
        .flat_map(|b| b.iter())
        .cloned()
        .collect_vec();
    assert_eq!(on_battlefield, [target.clone()]);
    assert_eq!(target.power(&db), Some(3));
    assert_eq!(target.toughness(&db), Some(3));
    assert_eq!(
        db[&target].modified_types,
        TypeSet::from([Type::CREATURE, Type::ARTIFACT,])
    );
    assert_eq!(
        db[&target].modified_subtypes,
        SubtypeSet::from([Subtype::SPIRIT])
    );

    Ok(())
}
