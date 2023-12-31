use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    in_play::{CardId, Database},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::Stack,
    turns::Phase,
};

#[test]
fn mace() -> anyhow::Result<()> {
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
    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, bear, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mace = CardId::upload(&mut db, &cards, player, "Mace of the Valiant");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, mace, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, mace, 0);
    // Pay the cost
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose the default only target
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(bear.power(&db), Some(4));
    assert_eq!(bear.toughness(&db), Some(2));

    let bear2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, bear2, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(bear.power(&db), Some(5));
    assert_eq!(bear.toughness(&db), Some(3));
    assert_eq!(bear2.power(&db), Some(4));
    assert_eq!(bear2.toughness(&db), Some(2));

    Ok(())
}
