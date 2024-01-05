use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::Database,
    in_play::{self, CardId, OnBattlefield},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    turns::Turn,
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
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player, "Thermal Blast");
    blast.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear }]],
        None,
        vec![],
    );

    let mut results = Stack::resolve_1(&mut db);
    dbg!(&results);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(bear.marked_damage(&db), 3);

    let mut results = Battlefield::check_sba(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), []);

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
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    for _ in 0..7 {
        let card = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
        card.move_to_graveyard(&mut db);
    }

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player, "Thermal Blast");
    blast.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear }]],
        None,
        vec![],
    );

    let mut results = Stack::resolve_1(&mut db);

    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(bear.marked_damage(&db), 5);

    let mut results = Battlefield::check_sba(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), []);

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
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    let other = all_players.new_player("Player".to_string(), 20);

    for _ in 0..7 {
        let card = CardId::upload(&mut db, &cards, other, "Alpine Grizzly");
        card.move_to_graveyard(&mut db);
    }

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player, "Thermal Blast");
    blast.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear }]],
        None,
        vec![],
    );

    let mut results = Stack::resolve_1(&mut db);

    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(bear.marked_damage(&db), 3);

    let mut results = Battlefield::check_sba(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), []);

    Ok(())
}
