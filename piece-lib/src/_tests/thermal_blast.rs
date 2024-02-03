use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    effects::{PendingEffects, SelectionResult},
    in_play::Database,
    in_play::{CardId, CastFrom},
    load_cards,
    player::AllPlayers,
    protogen::targets::Location,
    stack::{Selected, Stack, TargetType},
};

#[test]
fn damages_target() -> anyhow::Result<()> {
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

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player, "Thermal Blast");
    let mut results = PendingEffects::default();
    results.apply_results(blast.move_to_stack(
        &mut db,
        vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(bear),
            targeted: true,
            restrictions: vec![],
        }],
        CastFrom::Hand,
        vec![],
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    assert_eq!(bear.marked_damage(&db), 3);

    let mut results = Battlefields::check_sba(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
            .collect_vec(),
        Vec::<CardId>::default()
    );

    Ok(())
}

#[test]
fn damages_target_threshold() -> anyhow::Result<()> {
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

    for _ in 0..7 {
        let card = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
        card.move_to_graveyard(&mut db);
    }

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player, "Thermal Blast");
    let mut results = PendingEffects::default();
    results.apply_results(blast.move_to_stack(
        &mut db,
        vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(bear),
            targeted: true,
            restrictions: vec![],
        }],
        CastFrom::Hand,
        vec![],
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);

    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    assert_eq!(bear.marked_damage(&db), 5);

    let mut results = Battlefields::check_sba(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
            .collect_vec(),
        Vec::<CardId>::default()
    );

    Ok(())
}

#[test]
fn damages_target_threshold_other_player() -> anyhow::Result<()> {
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
    let other = all_players.new_player("Player".to_string(), 20);

    let mut db = Database::new(all_players);

    for _ in 0..7 {
        let card = CardId::upload(&mut db, &cards, other, "Alpine Grizzly");
        card.move_to_graveyard(&mut db);
    }

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player, "Thermal Blast");
    let mut results = PendingEffects::default();
    results.apply_results(blast.move_to_stack(
        &mut db,
        vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(bear),
            targeted: true,
            restrictions: vec![],
        }],
        CastFrom::Hand,
        vec![],
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);

    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    assert_eq!(bear.marked_damage(&db), 3);

    let mut results = Battlefields::check_sba(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
            .collect_vec(),
        Vec::<CardId>::default()
    );

    Ok(())
}
