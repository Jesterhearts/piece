use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    effects::{EffectBehaviors, PendingEffects, SelectionResult},
    in_play::{CardId, Database},
    library::Library,
    load_cards,
    player::AllPlayers,
    protogen::{effects::MoveToBattlefield, targets::Location},
    stack::{Selected, Stack, TargetType},
    turns::Phase,
};

#[test]
fn enters_tapped() -> anyhow::Result<()> {
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
    let mut db = Database::new(all_players);

    let card = CardId::upload(&mut db, &cards, player, "Krosan Verge");
    let mut results = PendingEffects::default();
    results.selected.push(Selected {
        location: Some(Location::ON_BATTLEFIELD),
        target_type: TargetType::Card(card),
        targeted: false,
        restrictions: vec![],
    });
    let to_apply = MoveToBattlefield::default().apply(&mut db, None, &mut results.selected, false);
    results.apply_results(to_apply);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(card.tapped(&db));

    Ok(())
}

#[test]
fn tutors() -> anyhow::Result<()> {
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
    let forest = CardId::upload(&mut db, &cards, player, "Forest");
    Library::place_on_top(&mut db, player, forest);

    let plains = CardId::upload(&mut db, &cards, player, "Plains");
    Library::place_on_top(&mut db, player, plains);

    let annul = CardId::upload(&mut db, &cards, player, "Annul");
    Library::place_on_top(&mut db, player, annul);

    let card = CardId::upload(&mut db, &cards, player, "Krosan Verge");
    let mut results = PendingEffects::default();
    results.selected.push(Selected {
        location: Some(Location::ON_BATTLEFIELD),
        target_type: TargetType::Card(card),
        targeted: false,
        restrictions: vec![],
    });
    let to_apply = MoveToBattlefield::default().apply(&mut db, None, &mut results.selected, false);
    results.apply_results(to_apply);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    card.untap(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 1);

    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::PendingChoice);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::PendingChoice);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(forest.is_in_location(&db, Location::ON_BATTLEFIELD));
    assert!(plains.is_in_location(&db, Location::ON_BATTLEFIELD));

    Ok(())
}
