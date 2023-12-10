use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
};

#[test]
fn aura_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack(&mut db, creature, vec![]);
    assert_eq!(results, []);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let results = Battlefield::add_from_stack(&mut db, aura, vec![creature]);
    assert_eq!(results, []);

    assert_eq!(creature.power(&mut db), Some(6));
    assert_eq!(creature.toughness(&mut db), Some(4));
    assert!(creature.vigilance(&mut db));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack(&mut db, card2, vec![]);
    assert_eq!(results, []);

    assert_eq!(card2.power(&mut db), Some(4));
    assert_eq!(card2.toughness(&mut db), Some(2));

    let results = Battlefield::permanent_to_graveyard(&mut db, aura);
    assert_eq!(results, []);

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
    let player = all_players.new_player();

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack(&mut db, creature, vec![]);
    assert_eq!(results, []);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let results = Battlefield::add_from_stack(&mut db, aura, vec![creature]);
    assert_eq!(results, []);

    assert_eq!(creature.power(&mut db), Some(6));
    assert_eq!(creature.toughness(&mut db), Some(4));
    assert!(creature.vigilance(&mut db));

    let results = Battlefield::check_sba(&mut db);
    assert_eq!(results, []);

    let results = Battlefield::permanent_to_graveyard(&mut db, creature);
    assert_eq!(results, []);

    let results = Battlefield::check_sba(&mut db);
    assert_eq!(results, [ActionResult::PermanentToGraveyard(aura)]);

    let results = Battlefield::apply_action_results(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    assert!(Battlefield::no_modifiers(&mut db));
    assert!(Battlefield::is_empty(&mut db));

    Ok(())
}

#[test]
fn vigilance_is_lost_no_green_permanent() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let creature = CardId::upload(&mut db, &cards, player, "Recruiter of the Guard");
    let _ = Battlefield::add_from_stack(&mut db, creature, vec![]);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let results = Battlefield::add_from_stack(&mut db, aura, vec![creature]);
    assert_eq!(results, []);

    assert_eq!(creature.power(&mut db), Some(3));
    assert_eq!(creature.toughness(&mut db), Some(3));
    assert!(!creature.vigilance(&mut db));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack(&mut db, card2, vec![]);
    assert_eq!(results, []);

    assert_eq!(card2.power(&mut db), Some(4));
    assert_eq!(card2.toughness(&mut db), Some(2));
    assert!(creature.vigilance(&mut db));

    let results = Battlefield::permanent_to_graveyard(&mut db, card2);
    assert_eq!(results, []);

    assert!(!creature.vigilance(&mut db));

    Ok(())
}
