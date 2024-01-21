use pretty_assertions::assert_eq;

use crate::{
    in_play::Database, load_cards, pending_results::ResolutionResult, player::AllPlayers,
    protogen::ids::CardId, stack::Stack, types::SubtypeSet,
};

#[test]
fn works() -> anyhow::Result<()> {
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

    let land = CardId::upload(&mut db, &cards, player.clone(), "Forest");
    land.move_to_battlefield(&mut db);

    let lithoform = CardId::upload(&mut db, &cards, player.clone(), "Lithoform Blight");
    let mut results = Stack::move_card_to_stack_from_hand(&mut db, lithoform, false);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db[&land].modified_subtypes, SubtypeSet::from([]));

    Ok(())
}
