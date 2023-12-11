use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield, in_play::CardId, in_play::Database, load_cards, player::AllPlayers,
};

#[test]
fn modifies_battlefield() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let elesh = CardId::upload(&mut db, &cards, player, "Elesh Norn, Grand Cenobite");
    let results = Battlefield::add_from_stack(&mut db, elesh, vec![]);
    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack(&mut db, bear, vec![]);
    assert_eq!(results, []);

    assert_eq!(elesh.power(&mut db), Some(4));
    assert_eq!(elesh.toughness(&mut db), Some(7));

    assert_eq!(bear.power(&mut db), Some(6));
    assert_eq!(bear.toughness(&mut db), Some(4));

    let results = Battlefield::permanent_to_graveyard(&mut db, elesh);
    assert_eq!(results, []);

    assert_eq!(bear.power(&mut db), Some(4));
    assert_eq!(bear.toughness(&mut db), Some(2));

    Ok(())
}
