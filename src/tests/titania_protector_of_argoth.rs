use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::{Stack, StackResult},
    types::Type,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_graveyard(&mut db);

    let titania = CardId::upload(&mut db, &cards, player, "Titania, Protector of Argoth");
    let results = Battlefield::add_from_stack(&mut db, titania, vec![]);
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
            source: titania,
            count: 1,
            types: HashSet::from([Type::Land, Type::BasicLand]),
            valid_targets: vec![land]
        }]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    Ok(())
}

#[test]
fn graveyard_trigger() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_battlefield(&mut db);

    let titania = CardId::upload(&mut db, &cards, player, "Titania, Protector of Argoth");
    let results = Battlefield::add_from_stack(&mut db, titania, vec![]);
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
            source: titania,
            count: 1,
            types: HashSet::from([Type::Land, Type::BasicLand]),
            valid_targets: vec![]
        }]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
            source: titania,
            count: 1,
            types: HashSet::from([Type::Land, Type::BasicLand]),
            valid_targets: vec![]
        }]
    );

    let results = Battlefield::permanent_to_graveyard(&mut db, land);
    assert!(matches!(
        results.as_slice(),
        [UnresolvedActionResult::AddTriggerToStack(_)]
    ));

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    let results = Stack::resolve_1(&mut db);
    assert!(matches!(
        results.as_slice(),
        [StackResult::CreateToken { .. }]
    ));
    let results = Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    assert_eq!(Battlefield::creatures(&mut db).len(), 2);

    Ok(())
}
