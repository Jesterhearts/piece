use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, in_play::Database, load_cards, pending_results::ResolutionResult,
    player::AllPlayers, protogen::ids::CardId,
};

#[test]
fn aura_works() -> anyhow::Result<()> {
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

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &creature, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &aura, Some(&creature));
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));
    assert!(creature.vigilance(&db));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &card2, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(card2.power(&db), Some(4));
    assert_eq!(card2.toughness(&db), Some(2));

    let results = Battlefields::permanent_to_graveyard(&mut db, &aura);
    assert!(results.is_empty());

    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));
    assert!(!creature.vigilance(&db));

    assert!(Battlefields::no_modifiers(&db));

    Ok(())
}

#[test]
fn aura_leaves_battlefield_enchanting_leaves_battlefield() -> anyhow::Result<()> {
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

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &creature, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &aura, Some(&creature));
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));
    assert!(creature.vigilance(&db));

    let results = Battlefields::check_sba(&mut db);
    assert!(results.is_empty());
    let results = Battlefields::permanent_to_graveyard(&mut db, &creature);
    assert!(results.is_empty());
    let mut results = Battlefields::check_sba(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Battlefields::no_modifiers(&db));
    assert!(db.battlefield.is_empty());

    Ok(())
}

#[test]
fn vigilance_is_lost_no_green_permanent() -> anyhow::Result<()> {
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

    let creature = CardId::upload(&mut db, &cards, player, "Recruiter of the Guard");
    let _ = Battlefields::add_from_stack_or_hand(&mut db, &creature, None);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &aura, Some(&creature));
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&db), Some(3));
    assert_eq!(creature.toughness(&db), Some(3));
    assert!(!creature.vigilance(&db));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &card2, None);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(card2.power(&db), Some(4));
    assert_eq!(card2.toughness(&db), Some(2));
    assert!(creature.vigilance(&db));

    let results = Battlefields::permanent_to_graveyard(&mut db, &card2);
    assert!(results.is_empty());
    assert!(!creature.vigilance(&db));

    Ok(())
}
