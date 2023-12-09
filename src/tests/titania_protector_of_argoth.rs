use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
    stack::{Stack, StackResult},
    types::Type,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let land = CardId::upload(&db, &cards, player, "Forest")?;
    land.move_to_graveyard(&db)?;

    let titania = CardId::upload(&db, &cards, player, "Titania, Protector of Argoth")?;
    let results = Battlefield::add(&db, titania, vec![])?;
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
            source: titania,
            count: 1,
            types: HashSet::from([Type::Land, Type::BasicLand]),
            valid_targets: vec![land]
        }]
    );

    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    Ok(())
}

#[test]
fn graveyard_trigger() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let land = CardId::upload(&db, &cards, player, "Forest")?;
    land.move_to_battlefield(&db)?;

    let titania = CardId::upload(&db, &cards, player, "Titania, Protector of Argoth")?;
    let results = Battlefield::add(&db, titania, vec![])?;
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
            source: titania,
            count: 1,
            types: HashSet::from([Type::Land, Type::BasicLand]),
            valid_targets: vec![]
        }]
    );

    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
            source: titania,
            count: 1,
            types: HashSet::from([Type::Land, Type::BasicLand]),
            valid_targets: vec![]
        }]
    );

    let results = Battlefield::permanent_to_graveyard(&db, land)?;
    assert!(matches!(
        results.as_slice(),
        [UnresolvedActionResult::AddTriggerToStack(_)]
    ));

    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    let results = Stack::resolve_1(&db)?;
    assert!(matches!(
        results.as_slice(),
        [StackResult::CreateToken { .. }]
    ));
    let results = Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    assert_eq!(Battlefield::creatures(&db)?.len(), 2);

    Ok(())
}
