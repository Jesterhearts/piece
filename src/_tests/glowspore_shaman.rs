use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield, in_play::CardId, in_play::Database, load_cards,
    pending_results::ResolutionResult, player::AllPlayers, stack::Stack, turns::Turn,
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
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    let nonland = CardId::upload(&mut db, &cards, player, "Annul");

    all_players[player].deck.place_on_top(&mut db, land);
    all_players[player].deck.place_on_top(&mut db, nonland);

    let glowspore = CardId::upload(&mut db, &cards, player, "Glowspore Shaman");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, glowspore, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Stack::is_empty(&mut db));
    assert_eq!(all_players[player].deck.len(), 1);

    Ok(())
}
