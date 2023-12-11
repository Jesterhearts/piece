use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
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
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack(&mut db, bear, vec![]);
    assert_eq!(results, []);

    let mace = CardId::upload(&mut db, &cards, player, "Mace of the Valiant");
    let results = Battlefield::add_from_stack(&mut db, mace, vec![]);
    assert_eq!(results, []);

    let results = Battlefield::activate_ability(&mut db, &mut all_players, mace, 0);
    assert!(matches!(
        results.as_slice(),
        [UnresolvedActionResult::AddAbilityToStack { .. }]
    ));
    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);
    let results = Stack::resolve_1(&mut db);
    Stack::apply_results(&mut db, &mut all_players, results);

    assert_eq!(bear.power(&mut db), Some(4));
    assert_eq!(bear.toughness(&mut db), Some(2));

    let bear2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack(&mut db, bear2, vec![]);
    assert!(matches!(
        results.as_slice(),
        [UnresolvedActionResult::AddTriggerToStack(_)]
    ));
    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);
    let results = Stack::resolve_1(&mut db);
    Stack::apply_results(&mut db, &mut all_players, results);

    assert_eq!(bear.power(&mut db), Some(5));
    assert_eq!(bear.toughness(&mut db), Some(3));
    assert_eq!(bear2.power(&mut db), Some(4));
    assert_eq!(bear2.toughness(&mut db), Some(2));

    Ok(())
}
