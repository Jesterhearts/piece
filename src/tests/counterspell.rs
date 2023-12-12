use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::{Entry, Stack, StackResult},
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
    let targets = HashSet::from([Stack::target_nth(&mut db, 0)]);
    counterspell_2.move_to_stack(&mut db, targets);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [
            StackResult::SpellCountered {
                id: Entry::Card(counterspell_1)
            },
            StackResult::StackToGraveyard(counterspell_2)
        ]
    );
    let results = Stack::apply_results(&mut db, &mut all_players, results);
    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    assert!(Stack::is_empty(&mut db));

    Ok(())
}
