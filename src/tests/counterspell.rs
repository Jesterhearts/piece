use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
    stack::{Entry, Stack, StackResult},
};

#[test]
fn resolves_counterspells() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let counterspell_1 = CardId::upload(&db, &cards, player, "Counterspell")?;
    let counterspell_2 = CardId::upload(&db, &cards, player, "Counterspell")?;

    counterspell_1.move_to_stack(&db, Default::default())?;
    counterspell_2.move_to_stack(&db, HashSet::from([Stack::target_nth(&db, 0)?]))?;

    assert_eq!(Stack::in_stack(&db)?.len(), 2);

    let results = Stack::resolve_1(&db)?;
    assert_eq!(
        results,
        [
            StackResult::SpellCountered {
                id: Entry::Card(counterspell_1)
            },
            StackResult::StackToGraveyard(counterspell_2)
        ]
    );
    let results = Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    assert!(Stack::is_empty(&db)?);

    Ok(())
}
