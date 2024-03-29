use pretty_assertions::assert_eq;

use crate::{
    effects::{PendingEffects, SelectionResult},
    in_play::{CardId, CastFrom, Database},
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn creates_tokens() -> anyhow::Result<()> {
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
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let card = CardId::upload(&mut db, &cards, player, "Forbidden Friendship");
    let mut results = PendingEffects::default();
    results.apply_results(card.move_to_stack(&mut db, vec![], CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .count(),
        2
    );

    Ok(())
}
