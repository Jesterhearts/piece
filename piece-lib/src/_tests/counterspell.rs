use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    effects::{PendingEffects, SelectionResult},
    in_play::Database,
    in_play::{CardId, CastFrom},
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn resolves_counterspells() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .pretty()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ENTER)
        .with_writer(std::io::stderr)
        .try_init();

    let cards = load_cards()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);

    let mut db = Database::new(all_players);

    let counterspell_1 = CardId::upload(&mut db, &cards, player, "Counterspell");
    let counterspell_2 = CardId::upload(&mut db, &cards, player, "Counterspell");

    let mut results = PendingEffects::default();
    results.apply_results(counterspell_1.move_to_stack(
        &mut db,
        Default::default(),
        CastFrom::Hand,
        vec![],
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let targets = vec![db.stack.target_nth(0)];
    results.apply_results(counterspell_2.move_to_stack(&mut db, targets, CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(db.stack.entries.len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(db.stack.is_empty());

    assert_eq!(
        db.graveyard[player],
        IndexSet::from([counterspell_1, counterspell_2])
    );

    Ok(())
}
