use indexmap::IndexSet;
use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, effects::SelectionResult, in_play::CardId, in_play::Database,
    load_cards, player::AllPlayers, stack::Stack, turns::Phase,
};

#[test]
fn destroys_artifact() -> anyhow::Result<()> {
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
    let player1 = all_players.new_player(String::default(), 20);
    all_players[player1].infinite_mana();
    let player2 = all_players.new_player(String::default(), 20);
    let mut db = Database::new(all_players);

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    card.move_to_battlefield(&mut db);
    let card2 = CardId::upload(&mut db, &cards, player1, "Deconstruction Hammer");
    card2.move_to_battlefield(&mut db);
    let card3 = CardId::upload(&mut db, &cards, player2, "Abzan Banner");
    card3.move_to_battlefield(&mut db);

    // Get rid of summoning sickness
    db.turn.turn_count += db.turn.turns_per_round();

    // Equip the bear
    let mut results = Battlefields::activate_ability(&mut db, &None, player1, card2, 0);
    // Pay the costs
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::PendingChoice);
    // End pay costs
    // Target the bear
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    // Resolve the equip
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    // Activate the ability on the bear, targeting the banner
    let mut results = Battlefields::activate_ability(&mut db, &None, player1, card, 0);
    // Pay the generic mana
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::PendingChoice);
    let result = results.resolve(&mut db, Some(1));
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    // Resolve the ability
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
            .collect_vec(),
        [card]
    );
    assert_eq!(db.graveyard[player1], IndexSet::from([card2]));
    assert_eq!(db.graveyard[player2], IndexSet::from([card3]));

    Ok(())
}
