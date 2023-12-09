use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
    stack::{Stack, StackResult},
};

#[test]
fn add_p_t_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let shade1 = CardId::upload(&db, &cards, player, "Hoar Shade")?;
    let shade2 = CardId::upload(&db, &cards, player, "Hoar Shade")?;

    let results = Battlefield::add_from_stack(&db, shade1, vec![])?;
    assert_eq!(results, []);

    let results = Battlefield::add_from_stack(&db, shade2, vec![])?;
    assert_eq!(results, []);

    let results = Battlefield::activate_ability(&db, &mut all_players, shade1, 0)?;

    assert_eq!(
        results,
        [UnresolvedActionResult::AddAbilityToStack {
            source: shade1,
            ability: shade1
                .activated_abilities(&db)?
                .first()
                .copied()
                .unwrap_or_default(),
            valid_targets: Default::default()
        }]
    );

    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    let results = Stack::resolve_1(&db)?;
    assert!(matches!(
        results.as_slice(),
        [StackResult::ApplyModifierToTarget { .. }]
    ));

    let results = Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    assert_eq!(shade1.power(&db)?, Some(2));
    assert_eq!(shade1.toughness(&db)?, Some(3));

    assert_eq!(shade2.power(&db)?, Some(1));
    assert_eq!(shade2.toughness(&db)?, Some(2));

    Battlefield::end_turn(&db)?;

    assert_eq!(shade1.power(&db)?, Some(1));
    assert_eq!(shade1.toughness(&db)?, Some(2));

    Ok(())
}
