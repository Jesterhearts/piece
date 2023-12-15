use std::collections::HashSet;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
};

#[test]
fn works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_owned(), 20);
    let mut db = Database::default();

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_battlefield(&mut db);

    let lithoform = CardId::upload(&mut db, &cards, player, "Lithoform Blight");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, lithoform);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(land.subtypes(&mut db), HashSet::default());

    Ok(())
}
