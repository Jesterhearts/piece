use pretty_assertions::assert_eq;

use crate::{
    effects::SelectionResult,
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    protogen::targets::Location,
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
    target.tap(&mut db);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // Target the bear
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // Pay white mana
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::PendingChoice);
    // Pay generic mana
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::PendingChoice);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(target.is_in_location(&db, Location::IN_EXILE));

    Ok(())
}
