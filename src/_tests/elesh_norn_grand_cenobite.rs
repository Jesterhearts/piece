use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    Battlefield,
};

#[test]
fn modifies_battlefield() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let elesh = CardId::upload(&mut db, &cards, player, "Elesh Norn, Grand Cenobite");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, elesh, None);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, bear, None);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(elesh.power(&db), Some(4));
    assert_eq!(elesh.toughness(&db), Some(7));

    assert_eq!(bear.power(&db), Some(6));
    assert_eq!(bear.toughness(&db), Some(4));

    let results = Battlefield::permanent_to_graveyard(&mut db, elesh);
    assert!(results.is_empty());
    assert_eq!(bear.power(&db), Some(4));
    assert_eq!(bear.toughness(&db), Some(2));

    Ok(())
}
