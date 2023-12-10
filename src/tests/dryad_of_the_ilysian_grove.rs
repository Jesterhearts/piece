use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    load_cards,
    player::{AllPlayers, Player},
    prepare_db,
    types::Subtype,
};

#[test]
fn adds_land_types() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let land = CardId::upload(&db, &cards, player, "Forest")?;
    let results = Battlefield::add_from_stack(&db, land, vec![])?;
    assert_eq!(results, []);

    let card = CardId::upload(&db, &cards, player, "Dryad of the Ilysian Grove")?;
    let results = Battlefield::add_from_stack(&db, card, vec![])?;
    assert!(matches!(
        results.as_slice(),
        [UnresolvedActionResult::AddModifier { .. }]
    ));
    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    assert_eq!(Player::lands_per_turn(&db)?, 2);

    assert_eq!(
        land.subtypes(&db)?,
        HashSet::from([
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest
        ])
    );

    Ok(())
}
