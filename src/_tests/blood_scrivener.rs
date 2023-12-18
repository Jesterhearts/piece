use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{self, CardId, Database, InHand},
    load_cards,
    player::AllPlayers,
};

#[test]
fn replacement() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let deck1 = CardId::upload(&mut db, &cards, player, "Annul");
    all_players[player].deck.place_on_top(&mut db, deck1);
    let deck2 = CardId::upload(&mut db, &cards, player, "Annul");
    all_players[player].deck.place_on_top(&mut db, deck2);

    let card = CardId::upload(&mut db, &cards, player, "Blood Scrivener");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Hand is empty
    let mut results = all_players[player].draw(&mut db, 1);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(all_players[player].life_total, 19);

    assert_eq!(in_play::cards::<InHand>(&mut db), [deck2, deck1]);

    Ok(())
}
