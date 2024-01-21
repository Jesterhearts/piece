use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    in_play::Database,
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    protogen::ids::CardId,
    stack::{ActiveTarget, Stack},
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
    all_players[&player].infinite_mana();

    let mut db = Database::new(all_players);

    let bear = CardId::upload(&mut db, &cards, player.clone(), "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player.clone(), "Thermal Blast");
    let mut results = blast.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear.clone() }]],
        None,
        vec![],
    );
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(bear.marked_damage(&db), 3);

    let mut results = Battlefields::check_sba(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .cloned()
            .collect_vec(),
        []
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
    all_players[&player].infinite_mana();

    let mut db = Database::new(all_players);

    for _ in 0..7 {
        let card = CardId::upload(&mut db, &cards, player.clone(), "Alpine Grizzly");
        card.move_to_graveyard(&mut db);
    }

    let bear = CardId::upload(&mut db, &cards, player.clone(), "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player.clone(), "Thermal Blast");
    let mut results = blast.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear.clone() }]],
        None,
        vec![],
    );
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);

    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(bear.marked_damage(&db), 5);

    let mut results = Battlefields::check_sba(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .cloned()
            .collect_vec(),
        []
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
    all_players[&player].infinite_mana();
    let other = all_players.new_player("Player".to_string(), 20);

    let mut db = Database::new(all_players);

    for _ in 0..7 {
        let card = CardId::upload(&mut db, &cards, other.clone(), "Alpine Grizzly");
        card.move_to_graveyard(&mut db);
    }

    let bear = CardId::upload(&mut db, &cards, player.clone(), "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player.clone(), "Thermal Blast");
    let mut results = blast.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear.clone() }]],
        None,
        vec![],
    );
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);

    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(bear.marked_damage(&db), 3);

    let mut results = Battlefields::check_sba(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .cloned()
            .collect_vec(),
        []
    );

    Ok(())
}
