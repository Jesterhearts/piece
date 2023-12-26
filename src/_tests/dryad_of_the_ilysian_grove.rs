use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::{AllPlayers, Player},
    turns::Turn,
    types::Subtype,
};

#[test]
fn adds_land_types() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&all_players);

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, land, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let card = CardId::upload(&mut db, &cards, player, "Dryad of the Ilysian Grove");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(Player::lands_per_turn(&mut db), 2);

    assert_eq!(
        land.subtypes(&db),
        IndexSet::from([
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest
        ])
    );

    Ok(())
}
