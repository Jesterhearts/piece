use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, in_play::Database, load_cards, pending_results::ResolutionResult,
    player::AllPlayers, protogen::ids::CardId, stack::Stack,
};

#[test]
fn ability() -> anyhow::Result<()> {
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
    let player = all_players.new_player("name".to_string(), 20);
    all_players[&player].infinite_mana();
    let mut db = Database::new(all_players);

    let card = CardId::upload(&mut db, &cards, player.clone(), "Deadapult");
    card.move_to_battlefield(&mut db);

    let sac = CardId::upload(&mut db, &cards, player.clone(), "Blood Scrivener");
    sac.move_to_battlefield(&mut db);

    let bear = CardId::upload(&mut db, &cards, player.clone(), "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, &player, &card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose to sacrifice the zombie
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Recompute targets
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay the generic mana
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose the bear as the target
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(bear.marked_damage(&db), 2);

    Ok(())
}
