use std::collections::HashSet;

use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::ResolutionResult,
    in_play::{self, CardId, Database, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    turns::Turn,
    types::{Subtype, Type},
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
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();

    let player = all_players.new_player("player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    let card = CardId::upload(&mut db, &cards, player, "Abuelo's Awakening");
    let target = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let non_target = CardId::upload(&mut db, &cards, player, "Abzan Runemark");

    target.move_to_graveyard(&mut db);
    non_target.move_to_graveyard(&mut db);

    assert_eq!(
        card.valid_targets(&mut db, &HashSet::default())[0],
        [ActiveTarget::Graveyard { id: target }]
    );

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card, true);
    // Choose the target
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay the white
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay the generic
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Skip the X
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let on_battlefield = in_play::cards::<OnBattlefield>(&mut db);
    assert_eq!(on_battlefield, [target]);
    assert_eq!(target.power(&mut db), Some(1));
    assert_eq!(target.toughness(&mut db), Some(1));
    assert_eq!(
        target.types(&db),
        IndexSet::from([Type::Creature, Type::Artifact])
    );
    assert_eq!(target.subtypes(&db), IndexSet::from([Subtype::Spirit]));

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
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();

    let player = all_players.new_player("player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    let card = CardId::upload(&mut db, &cards, player, "Abuelo's Awakening");
    let target = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let non_target = CardId::upload(&mut db, &cards, player, "Abzan Runemark");

    target.move_to_graveyard(&mut db);
    non_target.move_to_graveyard(&mut db);

    assert_eq!(
        card.valid_targets(&mut db, &HashSet::default())[0],
        [ActiveTarget::Graveyard { id: target }]
    );

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card, true);
    // Choose the target
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay the white
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay the generic
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // pay 1 for X
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    // pay 1 for X
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Skip paying x
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Add card to stack & pay
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let on_battlefield = in_play::cards::<OnBattlefield>(&mut db);
    assert_eq!(on_battlefield, [target]);
    assert_eq!(target.power(&mut db), Some(3));
    assert_eq!(target.toughness(&mut db), Some(3));
    assert_eq!(
        target.types(&db),
        IndexSet::from([Type::Creature, Type::Artifact])
    );
    assert_eq!(target.subtypes(&db), IndexSet::from([Subtype::Spirit]));

    Ok(())
}
