use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    controller::Controller,
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
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
    let nonland = CardId::upload(&db, &cards, player, "Annul")?;

    all_players[player].deck.place_on_top(&db, land)?;
    all_players[player].deck.place_on_top(&db, nonland)?;

    let glowspore = CardId::upload(&db, &cards, player, "Glowspore Shaman")?;
    let results = Battlefield::add(&db, glowspore, vec![])?;
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
                controller: Controller::You,
                types: HashSet::from([Type::Land, Type::BasicLand]),
                valid_targets: vec![]
            }
        ]
    );

    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToLibrary {
            source: glowspore,
            count: 1,
            controller: Controller::You,
            types: HashSet::from([Type::Land, Type::BasicLand]),
            valid_targets: vec![land]
        }]
    );

    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    Ok(())
}
