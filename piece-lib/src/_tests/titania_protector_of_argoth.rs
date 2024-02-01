use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack, SelectionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    protogen::{
        effects::{MoveToBattlefield, MoveToGraveyard},
        targets::Location,
    },
    stack::{Selected, Stack, TargetType},
};

#[test]
fn etb() -> anyhow::Result<()> {
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
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_graveyard(&mut db);

    let titania = CardId::upload(&mut db, &cards, player, "Titania, Protector of Argoth");
    let mut results = PendingEffects::default();
    results.apply_results(MoveToBattlefield::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(titania),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
            .collect_vec(),
        [titania, land]
    );

    Ok(())
}

#[test]
fn graveyard_trigger() -> anyhow::Result<()> {
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
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_battlefield(&mut db);

    let titania = CardId::upload(&mut db, &cards, player, "Titania, Protector of Argoth");
    let mut results = PendingEffects::default();
    results.apply_results(MoveToBattlefield::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(titania),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = PendingEffects::default();
    results.apply_results(MoveToGraveyard::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(land),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .count(),
        2
    );

    Ok(())
}
