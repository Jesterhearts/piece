use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, effects::SelectionResult, in_play::CardId, in_play::Database,
    load_cards, player::AllPlayers, stack::Stack, turns::Phase,
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
    equipment.move_to_battlefield(&mut db);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    creature.move_to_battlefield(&mut db);

    assert!(db[creature].abilities(&db).is_empty());

    let mut results = Battlefields::activate_ability(&mut db, &None, player, equipment, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(db[creature].abilities(&db).len(), 1);

    Ok(())
}
