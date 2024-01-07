use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield, in_play::CardId, in_play::Database, load_cards,
    pending_results::ResolutionResult, player::AllPlayers, stack::Stack, turns::Phase,
};

#[test]
fn adds_ability() -> anyhow::Result<()> {
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
    db.turn.set_phase(Phase::PreCombatMainPhase);
    let equipment = CardId::upload(&mut db, &cards, player, "Paradise Mantle");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, equipment, None);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, None);

    assert!(db[creature].abilities().is_empty());

    let mut results = Battlefield::activate_ability(&mut db, &None, player, equipment, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db[creature].abilities().len(), 1);

    Ok(())
}
