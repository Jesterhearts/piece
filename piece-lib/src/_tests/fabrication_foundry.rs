use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, in_play::Database, load_cards, pending_results::ResolutionResult,
    player::AllPlayers, protogen::ids::CardId, stack::Stack, turns::Phase,
};

#[test]
fn exile_return_to_battlefield() -> anyhow::Result<()> {
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

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player, "Fabrication Foundry");
    let gy = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let exiled = CardId::upload(&mut db, &cards, player, "Abzan Banner");

    card.move_to_battlefield(&mut db);
    gy.move_to_graveyard(&mut db);
    exiled.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, &card, 1);
    // Compute exile targets
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose exile card
    let result = results.resolve(&mut db, Some(1));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay white
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay generic
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose gy target
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Complete
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve ability
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db.battlefield[player], IndexSet::from([card, gy]));
    assert_eq!(db.exile[player], IndexSet::from([exiled]));

    Ok(())
}
