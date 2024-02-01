use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
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
    turns::Phase,
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
    equipment.move_to_battlefield(&mut db);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    creature.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, equipment, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));

    let creature2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = PendingEffects::default();
    results.apply_results(MoveToBattlefield::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::IN_HAND),
            target_type: TargetType::Card(creature2),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));

    assert_eq!(creature2.power(&db), Some(4));
    assert_eq!(creature2.toughness(&db), Some(2));

    let mut results = PendingEffects::default();
    results.apply_results(MoveToGraveyard::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(equipment),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));
    assert!(results.is_empty());
    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));

    assert!(Battlefields::no_modifiers(&db));

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
    equipment.move_to_battlefield(&mut db);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    creature.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, equipment, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));

    let creature2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    creature2.move_to_battlefield(&mut db);

    assert_eq!(creature2.power(&db), Some(4));
    assert_eq!(creature2.toughness(&db), Some(2));

    let mut results = Battlefields::activate_ability(&mut db, &None, player, equipment, 0);
    // Pay the generic
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, Some(1));
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));

    assert_eq!(creature2.power(&db), Some(6));
    assert_eq!(creature2.toughness(&db), Some(4));

    let mut results = PendingEffects::default();
    results.apply_results(MoveToGraveyard::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(equipment),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));
    assert!(results.is_empty());
    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));

    assert!(Battlefields::no_modifiers(&db));

    Ok(())
}
