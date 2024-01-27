use std::collections::VecDeque;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    effects::SelectionResult,
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::Phase,
};

#[test]
fn place_on_top() -> anyhow::Result<()> {
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
    let card = CardId::upload(&mut db, &cards, player, "King Crab");
    card.move_to_battlefield(&mut db);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    creature.move_to_battlefield(&mut db);

    // Get rid of summoning sickness
    db.turn.turn_count += db.turn.turns_per_round();

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 0);
    // Pay the blue
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::PendingChoice);
    // Pay the generic
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // Choose the default only target
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(
        db.all_players[player].library.cards,
        VecDeque::from([creature])
    );

    Ok(())
}
