use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, PendingResults, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn add_p_t_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let shade1 = CardId::upload(&mut db, &cards, player, "Hoar Shade");
    let shade2 = CardId::upload(&mut db, &cards, player, "Hoar Shade");

    let results = Battlefield::add_from_stack_or_hand(&mut db, shade1, vec![]);
    assert_eq!(results, PendingResults::default());

    let results = Battlefield::add_from_stack_or_hand(&mut db, shade2, vec![]);
    assert_eq!(results, PendingResults::default());

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, shade1, 0);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let results = Stack::resolve_1(&mut db);
    assert!(matches!(
        results.as_slice(),
        [ActionResult::ApplyModifierToTarget { .. }]
    ));

    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());

    assert_eq!(shade1.power(&db), Some(2));
    assert_eq!(shade1.toughness(&db), Some(3));

    assert_eq!(shade2.power(&db), Some(1));
    assert_eq!(shade2.toughness(&db), Some(2));

    Battlefield::end_turn(&mut db);

    assert_eq!(shade1.power(&db), Some(1));
    assert_eq!(shade1.toughness(&db), Some(2));

    Ok(())
}
