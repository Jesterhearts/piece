use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, PendingResults, ResolutionResult},
    in_play::Database,
    in_play::{cards, CardId, InExile},
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    types::Subtype,
    Battlefield,
};

#[test]
fn resolves_shift() -> anyhow::Result<()> {
    let all_cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let bear1 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");
    let bear2 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");
    let bear3 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");

    let results = Battlefield::add_from_stack_or_hand(&mut db, bear1);
    assert_eq!(results, PendingResults::default());
    let results = Battlefield::add_from_stack_or_hand(&mut db, bear2);
    assert_eq!(results, PendingResults::default());

    all_players[player].deck.place_on_top(&mut db, bear3);

    let shift = CardId::upload(&mut db, &all_cards, player, "Reality Shift");
    shift.move_to_stack(&mut db, vec![ActiveTarget::Battlefield { id: bear1 }]);

    let mut results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [
            ActionResult::ExileTarget(ActiveTarget::Battlefield { id: bear1 }),
            ActionResult::ManifestTopOfLibrary(player.into()),
            ActionResult::StackToGraveyard(shift),
        ]
        .into()
    );
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(cards::<InExile>(&mut db), [bear1]);

    assert_eq!(bear2.power(&db), Some(4));
    assert_eq!(bear2.toughness(&db), Some(2));
    assert_eq!(bear2.subtypes(&mut db), HashSet::from([Subtype::Bear]));

    assert_eq!(bear3.power(&db), Some(2));
    assert_eq!(bear3.toughness(&db), Some(2));
    assert_eq!(bear3.subtypes(&mut db), Default::default());

    Ok(())
}
