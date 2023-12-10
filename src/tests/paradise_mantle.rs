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
pub fn adds_ability() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let equipment = CardId::upload(&db, &cards, player, "Paradise Mantle")?;
    let _ = Battlefield::add_from_stack(&db, equipment, vec![])?;

    let creature = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let _ = Battlefield::add_from_stack(&db, creature, vec![])?;

    assert_eq!(creature.activated_abilities(&db)?, []);

    let results = Battlefield::activate_ability(&db, &mut all_players, equipment, 0)?;
    assert_eq!(
        results,
        [UnresolvedActionResult::AddAbilityToStack {
            source: equipment,
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

    assert_eq!(creature.activated_abilities(&db)?.len(), 1);

    Ok(())
}
