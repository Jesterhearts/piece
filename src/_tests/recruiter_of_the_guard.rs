use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::Database,
    in_play::{CardId, InHand},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::Turn,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&all_players);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    all_players[player].deck.place_on_top(&mut db, bear);

    let spell = CardId::upload(&mut db, &cards, player, "Annul");
    all_players[player].deck.place_on_top(&mut db, spell);

    let elesh = CardId::upload(&mut db, &cards, player, "Elesh Norn, Grand Cenobite");
    all_players[player].deck.place_on_top(&mut db, elesh);

    let recruiter = CardId::upload(&mut db, &cards, player, "Recruiter of the Guard");
    recruiter.move_to_hand(&mut db);
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, recruiter, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(all_players[player].deck.len(), 2);

    assert_eq!(player.get_cards::<InHand>(&mut db), [bear]);

    Ok(())
}
