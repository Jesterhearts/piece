use std::collections::VecDeque;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, PendingResult, PendingResults},
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
    let results = Battlefield::add_from_stack_or_hand(&mut db, card);
    assert_eq!(results, PendingResults::default());

    // Hand is empty
    let results = all_players[player].draw(&mut db, 1);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([PendingResult {
                apply_immediately: vec![ActionResult::LoseLife {
                    target: player.into(),
                    count: 1
                }],
                to_resolve: Default::default(),
                then_apply: vec![],
            }]),
            applied: false,
        }
    );

    assert_eq!(in_play::cards::<InHand>(&mut db), [deck2, deck1]);

    Ok(())
}
