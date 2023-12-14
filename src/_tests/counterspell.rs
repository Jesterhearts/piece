use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, PendingResults},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::{Entry, Stack},
    Battlefield,
};

#[test]
fn resolves_counterspells() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);

    let counterspell_1 = CardId::upload(&mut db, &cards, player, "Counterspell");
    let counterspell_2 = CardId::upload(&mut db, &cards, player, "Counterspell");

    counterspell_1.move_to_stack(&mut db, Default::default());
    let targets = vec![Stack::target_nth(&mut db, 0)];
    counterspell_2.move_to_stack(&mut db, targets);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [
            ActionResult::SpellCountered {
                id: Entry::Card(counterspell_1)
            },
            ActionResult::StackToGraveyard(counterspell_2)
        ]
    );
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());

    assert!(Stack::is_empty(&mut db));

    Ok(())
}
