use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield, in_play::CardId, in_play::Database, load_cards,
    pending_results::ResolutionResult, player::AllPlayers, stack::Stack, turns::Phase,
};

#[test]
fn equipment_works() -> anyhow::Result<()> {
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
    db.turn.set_phase(Phase::PreCombatMainPhase);
    let equipment = CardId::upload(&mut db, &cards, player, "+2 Mace");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, equipment, None);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, None);

    let mut results = Battlefield::activate_ability(&mut db, &None, player, equipment, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));

    let creature2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature2, None);

    assert_eq!(creature2.power(&db), Some(4));
    assert_eq!(creature2.toughness(&db), Some(2));

    let results = Battlefield::permanent_to_graveyard(&mut db, equipment);
    assert!(results.is_empty());
    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));

    assert!(Battlefield::no_modifiers(&db));

    Ok(())
}

#[test]
fn reequip_equipment_works() -> anyhow::Result<()> {
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
    db.turn.set_phase(Phase::PreCombatMainPhase);
    let equipment = CardId::upload(&mut db, &cards, player, "+2 Mace");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, equipment, None);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, None);

    let mut results = Battlefield::activate_ability(&mut db, &None, player, equipment, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));

    let creature2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature2, None);

    assert_eq!(creature2.power(&db), Some(4));
    assert_eq!(creature2.toughness(&db), Some(2));

    let mut results = Battlefield::activate_ability(&mut db, &None, player, equipment, 0);
    // Pay the generic
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, Some(1));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));

    assert_eq!(creature2.power(&db), Some(6));
    assert_eq!(creature2.toughness(&db), Some(4));

    let results = Battlefield::permanent_to_graveyard(&mut db, equipment);
    assert!(results.is_empty());
    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));

    assert!(Battlefield::no_modifiers(&db));

    Ok(())
}
