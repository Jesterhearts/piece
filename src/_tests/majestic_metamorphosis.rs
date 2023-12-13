use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, PendingResults},
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    types::{Subtype, Type},
    Battlefield,
};

#[test]
fn metamorphosis() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);

    let mantle = CardId::upload(&mut db, &cards, player, "Paradise Mantle");
    let results = Battlefield::add_from_stack_or_hand(&mut db, mantle, vec![]);
    assert_eq!(results, PendingResults::default());

    let majestic = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");

    majestic.move_to_stack(&mut db, vec![ActiveTarget::Battlefield { id: mantle }]);

    let results = Stack::resolve_1(&mut db);
    assert!(matches!(
        results.as_slice(),
        [
            ActionResult::ApplyModifierToTarget { .. },
            ActionResult::DrawCards { .. },
            ActionResult::StackToGraveyard { .. },
        ]
    ));

    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());

    assert_eq!(mantle.power(&db), Some(4));
    assert_eq!(mantle.toughness(&db), Some(4));
    assert_eq!(
        mantle.subtypes(&mut db),
        HashSet::from([Subtype::Equipment, Subtype::Angel])
    );
    assert_eq!(
        mantle.types(&mut db),
        HashSet::from([Type::Artifact, Type::Creature])
    );
    assert!(mantle.flying(&mut db));

    Ok(())
}

#[test]
fn metamorphosis_bear() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack_or_hand(&mut db, bear, vec![]);
    assert_eq!(results, PendingResults::default());

    let majestic = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");

    majestic.move_to_stack(&mut db, vec![ActiveTarget::Battlefield { id: bear }]);

    let results = Stack::resolve_1(&mut db);
    assert!(matches!(
        results.as_slice(),
        [
            ActionResult::ApplyModifierToTarget { .. },
            ActionResult::DrawCards { .. },
            ActionResult::StackToGraveyard { .. },
        ]
    ));

    let result = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(result, PendingResults::default());

    assert_eq!(bear.power(&db), Some(4));
    assert_eq!(bear.toughness(&db), Some(4));
    assert_eq!(
        bear.subtypes(&mut db),
        HashSet::from([Subtype::Bear, Subtype::Angel])
    );
    assert_eq!(
        bear.types(&mut db),
        HashSet::from([Type::Artifact, Type::Creature])
    );
    assert!(bear.flying(&mut db));

    Ok(())
}
