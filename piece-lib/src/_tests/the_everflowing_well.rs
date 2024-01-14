use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    in_play::{CardId, Database},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn copies_permanent() -> anyhow::Result<()> {
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

    let player = all_players.new_player("player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let card = CardId::upload(&mut db, &cards, player, "The Everflowing Well");
    card.move_to_battlefield(&mut db);
    card.transform(&mut db);

    let elesh = CardId::upload(&mut db, &cards, player, "Elesh Norn, Grand Cenobite");

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, elesh, true);
    // Spend the white mana
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Spend the myriad pools mana
    let result = results.resolve(&mut db, Some(2));
    assert_eq!(result, ResolutionResult::PendingChoice);
    dbg!(&db.stack.entries);
    // Fill in the rest of the generic mana
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    dbg!(&db.stack.entries);

    // Add the card to the stack
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    dbg!(&db.stack.entries);
    // Add the trigger to the stack
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    dbg!(&db.stack.entries);

    // Resolve the trigger
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    dbg!(results.options(&mut db));
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);
    dbg!(results.options(&mut db));

    assert_eq!(db[card].cloned_id, Some(elesh));

    Ok(())
}