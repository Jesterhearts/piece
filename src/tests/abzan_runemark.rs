use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield},
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
};

#[test]
fn aura_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let creature = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let results = Battlefield::add_from_stack(&db, creature, vec![])?;
    assert_eq!(results, []);

    let aura = CardId::upload(&db, &cards, player, "Abzan Runemark")?;
    let results = Battlefield::add_from_stack(&db, aura, vec![creature])?;
    assert_eq!(results, []);

    assert_eq!(creature.power(&db)?, Some(6));
    assert_eq!(creature.toughness(&db)?, Some(4));
    assert!(creature.vigilance(&db)?);

    let card2 = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let results = Battlefield::add_from_stack(&db, card2, vec![])?;
    assert_eq!(results, []);

    assert_eq!(card2.power(&db)?, Some(4));
    assert_eq!(card2.toughness(&db)?, Some(2));

    let results = Battlefield::permanent_to_graveyard(&db, aura)?;
    assert_eq!(results, []);

    assert_eq!(creature.power(&db)?, Some(4));
    assert_eq!(creature.toughness(&db)?, Some(2));
    assert!(!creature.vigilance(&db)?);

    assert!(Battlefield::no_modifiers(&db)?);

    Ok(())
}

#[test]
fn aura_leaves_battlefield_enchanting_leaves_battlefield() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let creature = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let results = Battlefield::add_from_stack(&db, creature, vec![])?;
    assert_eq!(results, []);

    let aura = CardId::upload(&db, &cards, player, "Abzan Runemark")?;
    let results = Battlefield::add_from_stack(&db, aura, vec![creature])?;
    assert_eq!(results, []);

    assert_eq!(creature.power(&db)?, Some(6));
    assert_eq!(creature.toughness(&db)?, Some(4));
    assert!(creature.vigilance(&db)?);

    let results = Battlefield::check_sba(&db)?;
    assert_eq!(results, []);

    let results = Battlefield::permanent_to_graveyard(&db, creature)?;
    assert_eq!(results, []);

    let results = Battlefield::check_sba(&db)?;
    assert_eq!(results, [ActionResult::PermanentToGraveyard(aura)]);

    let results = Battlefield::apply_action_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    assert!(Battlefield::no_modifiers(&db)?);
    assert!(Battlefield::is_empty(&db)?);

    Ok(())
}

#[test]
fn vigilance_is_lost_no_green_permanent() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let creature = CardId::upload(&db, &cards, player, "Recruiter of the Guard")?;
    let _ = Battlefield::add_from_stack(&db, creature, vec![])?;

    let aura = CardId::upload(&db, &cards, player, "Abzan Runemark")?;
    let results = Battlefield::add_from_stack(&db, aura, vec![creature])?;
    assert_eq!(results, []);

    assert_eq!(creature.power(&db)?, Some(3));
    assert_eq!(creature.toughness(&db)?, Some(3));
    assert!(!creature.vigilance(&db)?);

    let card2 = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let results = Battlefield::add_from_stack(&db, card2, vec![])?;
    assert_eq!(results, []);

    assert_eq!(card2.power(&db)?, Some(4));
    assert_eq!(card2.toughness(&db)?, Some(2));
    assert!(creature.vigilance(&db)?);

    let results = Battlefield::permanent_to_graveyard(&db, card2)?;
    assert_eq!(results, []);

    assert!(!creature.vigilance(&db)?);

    Ok(())
}
