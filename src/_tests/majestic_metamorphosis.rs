use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{PendingResults, ResolutionResult},
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
    let player = all_players.new_player("Player".to_string(), 20);

    let mantle = CardId::upload(&mut db, &cards, player, "Paradise Mantle");
    let results = Battlefield::add_from_stack_or_hand(&mut db, mantle);
    assert_eq!(results, PendingResults::default());

    let majestic = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");

    majestic.move_to_stack(&mut db, vec![ActiveTarget::Battlefield { id: mantle }]);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

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
    let player = all_players.new_player("Player".to_string(), 20);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack_or_hand(&mut db, bear);
    assert_eq!(results, PendingResults::default());

    let majestic = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");

    majestic.move_to_stack(&mut db, vec![ActiveTarget::Battlefield { id: bear }]);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

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
