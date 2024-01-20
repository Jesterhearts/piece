use pretty_assertions::assert_eq;

use crate::{
    in_play::Database,
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    protogen::{ids::CardId, targets::Location},
    stack::Stack,
};

#[test]
fn cost_reducer() -> anyhow::Result<()> {
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
    let player = all_players.new_player("".to_string(), 20);
    all_players[player].infinite_mana();
    let mut db = Database::new(all_players);

    let card = CardId::upload(&mut db, &cards, player, "Quicksand Whirlpool");

    let target = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    target.move_to_battlefield(&mut db);
    let mut results = target.tap(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card, true);
    // Target the bear
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Recompute the cost
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);

    // Pay white mana
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay 2 generic mana
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(target.is_in_location(&db, Location::IN_EXILE));

    Ok(())
}
