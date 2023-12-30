use pretty_assertions::assert_eq;

use crate::{
    battlefield::ResolutionResult,
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::Turn,
};

#[test]
fn p1p1() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&all_players);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    creature.move_to_battlefield(&mut db);

    let card = CardId::upload(&mut db, &cards, player, "Family Reunion");
    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card, true);
    // Choose the mode
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay the white
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay the generic
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.power(&mut db), Some(5));
    assert_eq!(creature.toughness(&mut db), Some(3));

    Ok(())
}

#[test]
fn hexproof() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&all_players);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    creature.move_to_battlefield(&mut db);

    let card = CardId::upload(&mut db, &cards, player, "Family Reunion");
    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card, true);
    // Choose the mode
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(1));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay the white
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay the generic
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(creature.hexproof(&mut db));

    Ok(())
}
