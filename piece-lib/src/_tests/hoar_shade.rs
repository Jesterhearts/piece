use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, effects::SelectionResult, in_play::CardId, in_play::Database,
    load_cards, player::AllPlayers, stack::Stack, turns::Phase,
};

#[test]
fn add_p_t_works() -> anyhow::Result<()> {
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
    let shade1 = CardId::upload(&mut db, &cards, player, "Hoar Shade");
    shade1.move_to_battlefield(&mut db);
    let shade2 = CardId::upload(&mut db, &cards, player, "Hoar Shade");
    shade2.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, shade1, 0);
    // Pay Costs
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // End pay costs
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(shade1.power(&db), Some(2));
    assert_eq!(shade1.toughness(&db), Some(3));

    assert_eq!(shade2.power(&db), Some(1));
    assert_eq!(shade2.toughness(&db), Some(2));

    let mut results = Battlefields::end_turn(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(shade1.power(&db), Some(1));
    assert_eq!(shade1.toughness(&db), Some(2));

    Ok(())
}
