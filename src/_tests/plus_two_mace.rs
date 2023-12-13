use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, PendingResults, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn equipment_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let equipment = CardId::upload(&mut db, &cards, player, "+2 Mace");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, equipment, vec![]);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, vec![]);

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, equipment, 0);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let results = Stack::resolve_1(&mut db);
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));

    let creature2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature2, vec![]);

    assert_eq!(creature2.power(&db), Some(4));
    assert_eq!(creature2.toughness(&db), Some(2));

    let results = Battlefield::permanent_to_graveyard(&mut db, equipment);
    assert_eq!(results, PendingResults::default());

    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));

    assert!(Battlefield::no_modifiers(&mut db));

    Ok(())
}

#[test]
fn reequip_equipment_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let equipment = CardId::upload(&mut db, &cards, player, "+2 Mace");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, equipment, vec![]);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, vec![]);

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, equipment, 0);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let results = Stack::resolve_1(&mut db);
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));

    let creature2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature2, vec![]);

    assert_eq!(creature2.power(&db), Some(4));
    assert_eq!(creature2.toughness(&db), Some(2));

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, equipment, 0);
    let result = results.resolve(&mut db, &mut all_players, Some(1));
    assert_eq!(result, ResolutionResult::Complete);

    let results = Stack::resolve_1(&mut db);
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());

    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));

    assert_eq!(creature2.power(&db), Some(6));
    assert_eq!(creature2.toughness(&db), Some(4));

    let results = Battlefield::permanent_to_graveyard(&mut db, equipment);
    assert_eq!(results, PendingResults::default());

    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));

    assert!(Battlefield::no_modifiers(&mut db));

    Ok(())
}
