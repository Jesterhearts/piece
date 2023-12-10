use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::Database,
    in_play::{cards, CardId, InExile},
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack, StackResult},
    types::Subtype,
};

#[test]
fn resolves_shift() -> anyhow::Result<()> {
    let all_cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let bear1 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");
    let bear2 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");
    let bear3 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");

    let results = Battlefield::add_from_stack(&mut db, bear1, vec![]);
    assert_eq!(results, []);
    let results = Battlefield::add_from_stack(&mut db, bear2, vec![]);
    assert_eq!(results, []);

    all_players[player].deck.place_on_top(&mut db, bear3);

    let shift = CardId::upload(&mut db, &all_cards, player, "Reality Shift");
    shift.move_to_stack(
        &mut db,
        HashSet::from([ActiveTarget::Battlefield { id: bear1 }]),
    );

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [
            StackResult::ExileTarget(bear1),
            StackResult::ManifestTopOfLibrary(player.into()),
            StackResult::StackToGraveyard(shift),
        ]
    );
    let results = Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    assert_eq!(cards::<InExile>(&mut db), [bear1]);

    assert_eq!(bear2.power(&mut db), Some(4));
    assert_eq!(bear2.toughness(&mut db), Some(2));
    assert_eq!(bear2.subtypes(&mut db), HashSet::from([Subtype::Bear]));

    assert_eq!(bear3.power(&mut db), Some(2));
    assert_eq!(bear3.toughness(&mut db), Some(2));
    assert_eq!(bear3.subtypes(&mut db), Default::default());

    Ok(())
}
