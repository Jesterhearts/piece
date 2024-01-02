use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::{Phase, Turn},
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
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let mut turn = Turn::new(&mut db, &all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let equipment = CardId::upload(&mut db, &cards, player, "+2 Mace");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, equipment, None);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, None);

    let mut results = Battlefield::activate_ability(
        &mut db,
        &mut all_players,
        &turn,
        &None,
        player,
        equipment,
        0,
    );
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&mut db), Some(6));
    assert_eq!(creature.toughness(&mut db), Some(4));

    let creature2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature2, None);

    assert_eq!(creature2.power(&mut db), Some(4));
    assert_eq!(creature2.toughness(&mut db), Some(2));

    let results = Battlefield::permanent_to_graveyard(&mut db, &turn, equipment);
    assert!(results.is_empty());
    assert_eq!(creature.power(&mut db), Some(4));
    assert_eq!(creature.toughness(&mut db), Some(2));

    assert!(Battlefield::no_modifiers(&mut db));

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
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let mut turn = Turn::new(&mut db, &all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let equipment = CardId::upload(&mut db, &cards, player, "+2 Mace");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, equipment, None);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, None);

    let mut results = Battlefield::activate_ability(
        &mut db,
        &mut all_players,
        &turn,
        &None,
        player,
        equipment,
        0,
    );
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&mut db), Some(6));
    assert_eq!(creature.toughness(&mut db), Some(4));

    let creature2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature2, None);

    assert_eq!(creature2.power(&mut db), Some(4));
    assert_eq!(creature2.toughness(&mut db), Some(2));

    let mut results = Battlefield::activate_ability(
        &mut db,
        &mut all_players,
        &turn,
        &None,
        player,
        equipment,
        0,
    );
    // Pay the generic
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(1));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&mut db), Some(4));
    assert_eq!(creature.toughness(&mut db), Some(2));

    assert_eq!(creature2.power(&mut db), Some(6));
    assert_eq!(creature2.toughness(&mut db), Some(4));

    let results = Battlefield::permanent_to_graveyard(&mut db, &turn, equipment);
    assert!(results.is_empty());
    assert_eq!(creature.power(&mut db), Some(4));
    assert_eq!(creature.toughness(&mut db), Some(2));

    assert!(Battlefield::no_modifiers(&mut db));

    Ok(())
}
