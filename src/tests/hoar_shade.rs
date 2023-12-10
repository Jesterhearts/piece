use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::{Stack, StackResult},
};

#[test]
fn add_p_t_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let shade1 = CardId::upload(&mut db, &cards, player, "Hoar Shade");
    let shade2 = CardId::upload(&mut db, &cards, player, "Hoar Shade");

    let results = Battlefield::add_from_stack(&mut db, shade1, vec![]);
    assert_eq!(results, []);

    let results = Battlefield::add_from_stack(&mut db, shade2, vec![]);
    assert_eq!(results, []);

    let results = Battlefield::activate_ability(&mut db, &mut all_players, shade1, 0);

    assert_eq!(
        results,
        [UnresolvedActionResult::AddAbilityToStack {
            source: shade1,
            ability: shade1
                .activated_abilities(&mut db)
                .first()
                .copied()
                .unwrap(),
            valid_targets: Default::default()
        }]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    let results = Stack::resolve_1(&mut db);
    assert!(matches!(
        results.as_slice(),
        [StackResult::ApplyModifierToTarget { .. }]
    ));

    let results = Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    assert_eq!(shade1.power(&mut db), Some(2));
    assert_eq!(shade1.toughness(&mut db), Some(3));

    assert_eq!(shade2.power(&mut db), Some(1));
    assert_eq!(shade2.toughness(&mut db), Some(2));

    Battlefield::end_turn(&mut db);

    assert_eq!(shade1.power(&mut db), Some(1));
    assert_eq!(shade1.toughness(&mut db), Some(2));

    Ok(())
}
