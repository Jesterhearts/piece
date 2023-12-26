use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    card::Color,
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

    let aura = CardId::upload(&mut db, &cards, player, "Sinister Strength");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, aura, Some(creature));
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&db), Some(7));
    assert_eq!(creature.toughness(&db), Some(3));
    assert_eq!(creature.colors(&db), HashSet::from([Color::Black]));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(card2.power(&db), Some(4));
    assert_eq!(card2.toughness(&db), Some(2));

    let results = Battlefield::permanent_to_graveyard(&mut db, &turn, aura);
    assert!(results.is_empty());
    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));
    assert_eq!(creature.colors(&db), HashSet::from([Color::Green]));

    assert!(Battlefield::no_modifiers(&mut db));

    Ok(())
}
