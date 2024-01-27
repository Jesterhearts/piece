use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    effects::SelectionResult,
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    protogen::{
        mana::ManaSource,
        mana::{Mana, ManaRestriction},
    },
    turns::Phase,
};

#[test]
fn plains() -> anyhow::Result<()> {
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

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player, "Plains");
    card.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(db.stack.is_empty());

    assert_eq!(
        db.all_players[player].mana_pool.all_mana().collect_vec(),
        [
            (1, Mana::WHITE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLUE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLACK, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::RED, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::GREEN, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::COLORLESS, ManaSource::ANY, ManaRestriction::NONE),
        ]
    );

    Ok(())
}

#[test]
fn island() -> anyhow::Result<()> {
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

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player, "Island");
    card.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(db.stack.is_empty());
    assert_eq!(
        db.all_players[player].mana_pool.all_mana().collect_vec(),
        [
            (0, Mana::WHITE, ManaSource::ANY, ManaRestriction::NONE),
            (1, Mana::BLUE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLACK, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::RED, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::GREEN, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::COLORLESS, ManaSource::ANY, ManaRestriction::NONE),
        ]
    );

    Ok(())
}

#[test]
fn swamp() -> anyhow::Result<()> {
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

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player, "Swamp");
    card.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(db.stack.is_empty());

    assert_eq!(
        db.all_players[player].mana_pool.all_mana().collect_vec(),
        [
            (0, Mana::WHITE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLUE, ManaSource::ANY, ManaRestriction::NONE),
            (1, Mana::BLACK, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::RED, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::GREEN, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::COLORLESS, ManaSource::ANY, ManaRestriction::NONE),
        ]
    );

    Ok(())
}

#[test]
fn mountain() -> anyhow::Result<()> {
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

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player, "Mountain");
    card.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(db.stack.is_empty());
    assert_eq!(
        db.all_players[player].mana_pool.all_mana().collect_vec(),
        [
            (0, Mana::WHITE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLUE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLACK, ManaSource::ANY, ManaRestriction::NONE),
            (1, Mana::RED, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::GREEN, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::COLORLESS, ManaSource::ANY, ManaRestriction::NONE),
        ]
    );

    Ok(())
}

#[test]
fn forest() -> anyhow::Result<()> {
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

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player, "Forest");
    card.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(db.stack.is_empty());
    assert_eq!(
        db.all_players[player].mana_pool.all_mana().collect_vec(),
        [
            (0, Mana::WHITE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLUE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLACK, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::RED, ManaSource::ANY, ManaRestriction::NONE),
            (1, Mana::GREEN, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::COLORLESS, ManaSource::ANY, ManaRestriction::NONE),
        ]
    );

    Ok(())
}
