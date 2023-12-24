use pretty_assertions::assert_eq;

use crate::{
    battlefield::ResolutionResult,
    in_play::Database,
    in_play::{self, CardId, InGraveyard},
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn resolves_counterspells() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);

    let counterspell_1 = CardId::upload(&mut db, &cards, player, "Counterspell");
    let counterspell_2 = CardId::upload(&mut db, &cards, player, "Counterspell");

    counterspell_1.move_to_stack(&mut db, Default::default(), None, vec![]);
    let targets = vec![vec![Stack::target_nth(&mut db, 0)]];
    counterspell_2.move_to_stack(&mut db, targets, None, vec![]);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Stack::is_empty(&mut db));

    assert_eq!(
        in_play::cards::<InGraveyard>(&mut db),
        [counterspell_1, counterspell_2]
    );

    Ok(())
}
