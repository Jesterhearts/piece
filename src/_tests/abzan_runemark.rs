use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    turns::Turn,
};

#[test]
fn aura_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let turn = Turn::new(&all_players);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, creature, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, aura, Some(creature));
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&mut db), Some(6));
    assert_eq!(creature.toughness(&mut db), Some(4));
    assert!(creature.vigilance(&mut db));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(card2.power(&mut db), Some(4));
    assert_eq!(card2.toughness(&mut db), Some(2));

    let results = Battlefield::permanent_to_graveyard(&mut db, &turn, aura);
    assert!(results.is_empty());

    assert_eq!(creature.power(&mut db), Some(4));
    assert_eq!(creature.toughness(&mut db), Some(2));
    assert!(!creature.vigilance(&mut db));

    assert!(Battlefield::no_modifiers(&mut db));

    Ok(())
}

#[test]
fn aura_leaves_battlefield_enchanting_leaves_battlefield() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let turn = Turn::new(&all_players);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, creature, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, aura, Some(creature));
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&mut db), Some(6));
    assert_eq!(creature.toughness(&mut db), Some(4));
    assert!(creature.vigilance(&mut db));

    let results = Battlefield::check_sba(&mut db);
    assert!(results.is_empty());
    let results = Battlefield::permanent_to_graveyard(&mut db, &turn, creature);
    assert!(results.is_empty());
    let mut results = Battlefield::check_sba(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Battlefield::no_modifiers(&mut db));
    assert!(Battlefield::is_empty(&mut db));

    Ok(())
}

#[test]
fn vigilance_is_lost_no_green_permanent() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let turn = Turn::new(&all_players);

    let creature = CardId::upload(&mut db, &cards, player, "Recruiter of the Guard");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, None);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, aura, Some(creature));
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&mut db), Some(3));
    assert_eq!(creature.toughness(&mut db), Some(3));
    assert!(!creature.vigilance(&mut db));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(card2.power(&mut db), Some(4));
    assert_eq!(card2.toughness(&mut db), Some(2));
    assert!(creature.vigilance(&mut db));

    let results = Battlefield::permanent_to_graveyard(&mut db, &turn, card2);
    assert!(results.is_empty());
    assert!(!creature.vigilance(&mut db));

    Ok(())
}
