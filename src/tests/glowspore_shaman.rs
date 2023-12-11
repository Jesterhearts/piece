use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    controller::ControllerRestriction,
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    types::Type,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    let nonland = CardId::upload(&mut db, &cards, player, "Annul");

    all_players[player].deck.place_on_top(&mut db, land);
    all_players[player].deck.place_on_top(&mut db, nonland);

    let glowspore = CardId::upload(&mut db, &cards, player, "Glowspore Shaman");
    let results = Battlefield::add_from_stack(&mut db, glowspore, vec![]);
    assert_eq!(
        results,
        [
            UnresolvedActionResult::Mill {
                count: 3,
                valid_targets: HashSet::from([player])
            },
            UnresolvedActionResult::ReturnFromGraveyardToLibrary {
                source: glowspore,
                count: 1,
                controller: ControllerRestriction::You,
                types: HashSet::from([Type::Land, Type::BasicLand]),
                valid_targets: vec![]
            }
        ]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToLibrary {
            source: glowspore,
            count: 1,
            controller: ControllerRestriction::You,
            types: HashSet::from([Type::Land, Type::BasicLand]),
            valid_targets: vec![land]
        }]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    Ok(())
}
