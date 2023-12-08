use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
    stack::{ActiveTarget, Stack, StackResult},
};

#[test]
fn equipment_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let equipment = CardId::upload(&db, &cards, player, "+2 Mace")?;
    let _ = Battlefield::add(&db, equipment, vec![])?;

    let creature = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let _ = Battlefield::add(&db, creature, vec![])?;

    let results = Battlefield::activate_ability(&db, &mut all_players, equipment, 0)?;
    assert_eq!(
        results,
        [UnresolvedActionResult::AddAbilityToStack {
            ability: equipment
                .activated_abilities(&db)?
                .first()
                .copied()
                .unwrap_or_default(),
            valid_targets: HashSet::from([ActiveTarget::Battlefield { id: creature }])
        }]
    );

    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    let results = Stack::resolve_1(&db)?;
    assert!(matches!(
        results.as_slice(),
        [StackResult::ModifyCreatures { .. }]
    ));

    let results = Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    assert_eq!(creature.power(&db)?, Some(6));
    assert_eq!(creature.toughness(&db)?, Some(4));

    let creature2 = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let _ = Battlefield::add(&db, creature2, vec![])?;

    assert_eq!(creature2.power(&db)?, Some(4));
    assert_eq!(creature2.toughness(&db)?, Some(2));

    let results = Battlefield::permanent_to_graveyard(&db, equipment)?;
    assert_eq!(results, []);

    assert_eq!(creature.power(&db)?, Some(4));
    assert_eq!(creature.toughness(&db)?, Some(2));

    assert!(Battlefield::no_modifiers(&db)?);

    Ok(())
}
