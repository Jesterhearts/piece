use std::collections::HashSet;


use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn creates_tokens() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let card = CardId::upload(&mut db, &cards, player, "Forbidden Friendship");
    let targets = card.valid_targets(&mut db, &HashSet::default());
    card.move_to_stack(&mut db, targets, None);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(Battlefield::creatures(&mut db).len(), 2);

    Ok(())
}
