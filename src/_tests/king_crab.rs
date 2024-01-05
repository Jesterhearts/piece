use std::collections::{HashSet, VecDeque};

use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::{CardId, Database},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    turns::{Phase, Turn},
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
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut turn = Turn::new(&mut db, &all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "King Crab");
    card.move_to_battlefield(&mut db);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    creature.move_to_battlefield(&mut db);

    assert_eq!(
        card.valid_targets(&mut db, &HashSet::default()),
        vec![vec![ActiveTarget::Battlefield { id: creature }]]
    );

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, &None, player, card, 0);
    // Pay the blue
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay the generic
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose the default only target
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(all_players[player].deck.cards, VecDeque::from([creature]));

    Ok(())
}
