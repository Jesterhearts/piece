use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack, StackResult},
};

#[test]
pub fn adds_ability() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let equipment = CardId::upload(&mut db, &cards, player, "Paradise Mantle");
    let _ = Battlefield::add_from_stack(&mut db, equipment, vec![]);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack(&mut db, creature, vec![]);

    assert_eq!(creature.activated_abilities(&mut db), []);

    let results = Battlefield::activate_ability(&mut db, &mut all_players, equipment, 0);
    assert_eq!(
        results,
        [UnresolvedActionResult::AddAbilityToStack {
            source: equipment,
            ability: equipment
                .activated_abilities(&mut db)
                .first()
                .copied()
                .unwrap(),
            valid_targets: HashSet::from([ActiveTarget::Battlefield { id: creature }])
        }]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    let results = Stack::resolve_1(&mut db);
    assert!(matches!(
        results.as_slice(),
        [StackResult::ModifyCreatures { .. }]
    ));

    let results = Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    assert_eq!(creature.activated_abilities(&mut db).len(), 1);

    Ok(())
}
