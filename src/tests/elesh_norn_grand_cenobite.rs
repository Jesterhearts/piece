use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield, in_play::CardId, load_cards, player::AllPlayers, prepare_db,
};

#[test]
fn modifies_battlefield() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let elesh = CardId::upload(&db, &cards, player, "Elesh Norn, Grand Cenobite")?;
    let results = Battlefield::add(&db, elesh, vec![])?;
    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    let bear = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let results = Battlefield::add(&db, bear, vec![])?;
    assert_eq!(results, []);

    assert_eq!(elesh.power(&db)?, Some(4));
    assert_eq!(elesh.toughness(&db)?, Some(7));

    assert_eq!(bear.power(&db)?, Some(6));
    assert_eq!(bear.toughness(&db)?, Some(4));

    let results = Battlefield::permanent_to_graveyard(&db, elesh)?;
    assert_eq!(results, []);

    assert_eq!(bear.power(&db)?, Some(4));
    assert_eq!(bear.toughness(&db)?, Some(2));

    Ok(())
}
