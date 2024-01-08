use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, in_play::CardId, in_play::Database, library::Library, load_cards,
    pending_results::ResolutionResult, player::AllPlayers, stack::Stack,
};

#[test]
fn etb() -> anyhow::Result<()> {
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

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    let nonland = CardId::upload(&mut db, &cards, player, "Annul");

    Library::place_on_top(&mut db, player, land);
    Library::place_on_top(&mut db, player, nonland);

    let glowspore = CardId::upload(&mut db, &cards, player, "Glowspore Shaman");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, glowspore, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(db.stack.is_empty());
    assert_eq!(db.all_players[player].library.len(), 1);

    Ok(())
}
