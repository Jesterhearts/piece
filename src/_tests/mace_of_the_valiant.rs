use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, PendingResults, ResolutionResult},
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn mace() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack_or_hand(&mut db, bear, vec![]);
    assert_eq!(results, PendingResults::default());

    let mace = CardId::upload(&mut db, &cards, player, "Mace of the Valiant");
    let results = Battlefield::add_from_stack_or_hand(&mut db, mace, vec![]);
    assert_eq!(results, PendingResults::default());

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, mace, 0);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let results = Stack::resolve_1(&mut db);
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());

    assert_eq!(bear.power(&db), Some(4));
    assert_eq!(bear.toughness(&db), Some(2));

    let bear2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, bear2, vec![]);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let results = Stack::resolve_1(&mut db);
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());

    assert_eq!(bear.power(&db), Some(5));
    assert_eq!(bear.toughness(&db), Some(3));
    assert_eq!(bear2.power(&db), Some(4));
    assert_eq!(bear2.toughness(&db), Some(2));

    Ok(())
}