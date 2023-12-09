use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::{CardId, Location},
    load_cards,
    player::AllPlayers,
    prepare_db,
    stack::{ActiveTarget, Stack, StackResult},
    types::Subtype,
};

#[test]
fn resolves_shift() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let bear1 = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let bear2 = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let bear3 = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;

    let results = Battlefield::add(&db, bear1, vec![])?;
    assert_eq!(results, []);
    let results = Battlefield::add(&db, bear2, vec![])?;
    assert_eq!(results, []);

    all_players[player].deck.place_on_top(&db, bear3)?;

    let shift = CardId::upload(&db, &cards, player, "Reality Shift")?;
    shift.move_to_stack(
        &db,
        HashSet::from([ActiveTarget::Battlefield { id: bear1 }]),
    )?;

    let results = Stack::resolve_1(&db)?;
    assert_eq!(
        results,
        [
            StackResult::ExileTarget(bear1),
            StackResult::ManifestTopOfLibrary(player),
            StackResult::StackToGraveyard(shift),
        ]
    );
    let results = Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    assert_eq!(Location::Exile.cards_in(&db)?, [bear1]);

    assert_eq!(bear2.power(&db)?, Some(4));
    assert_eq!(bear2.toughness(&db)?, Some(2));
    assert_eq!(bear2.subtypes(&db)?, HashSet::from([Subtype::Bear]));

    assert_eq!(bear3.power(&db)?, Some(2));
    assert_eq!(bear3.toughness(&db)?, Some(2));
    assert_eq!(bear3.subtypes(&db)?, Default::default());

    Ok(())
}
