use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    effects::SelectionResult,
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    protogen::types::Type,
    stack::Stack,
    turns::Phase,
    types::TypeSet,
};

#[test]
fn spawns_bats() -> anyhow::Result<()> {
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
    let player = all_players.new_player(String::default(), 20);
    let mut db = Database::new(all_players);

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let cave1 = CardId::upload(&mut db, &cards, player, "Hidden Courtyard");
    cave1.move_to_battlefield(&mut db);
    let cave2 = CardId::upload(&mut db, &cards, player, "Hidden Courtyard");
    cave2.move_to_battlefield(&mut db);
    let cave3 = CardId::upload(&mut db, &cards, player, "Hidden Courtyard");
    cave3.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, cave1, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    let mut results = Battlefields::activate_ability(&mut db, &None, player, cave2, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    let mut results = Battlefields::activate_ability(&mut db, &None, player, cave3, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let bat_colony = CardId::upload(&mut db, &cards, player, "Bat Colony");
    let mut results = Stack::move_card_to_stack_from_hand(&mut db, bat_colony);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // Pay white
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::PendingChoice);
    // Pay generic
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // Cast card
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    // Resolve card
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    // Resolve etb
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    // Should have 3 bats
    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
            .filter(|card| card.types_intersect(&db, &TypeSet::from([Type::CREATURE])))
            .count(),
        3
    );

    Ok(())
}
