use indexmap::IndexSet;
use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::{CardId, Database},
    library::Library,
    load_cards,
    mana::{Mana, ManaRestriction},
    pending_results::ResolutionResult,
    player::{mana_pool::ManaSource, AllPlayers},
    stack::Stack,
    turns::Phase,
};

#[test]
fn sacrifice_draw_card() -> anyhow::Result<()> {
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

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    Library::place_on_top(&mut db, player, land);

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::activate_ability(&mut db, &None, player, card, 1);
    // Pay banner cost
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // End pay banner costs
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db.graveyard[player], IndexSet::from([card]));
    assert_eq!(db.hand[player], IndexSet::from([land]));

    Ok(())
}

#[test]
fn add_mana() -> anyhow::Result<()> {
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
    db.turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::activate_ability(&mut db, &None, player, card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);

    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        db.all_players[player].mana_pool.all_mana().collect_vec(),
        [
            (1, Mana::White, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Blue, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Black, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Red, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Green, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Colorless, ManaSource::Any, ManaRestriction::None),
        ]
    );

    Ok(())
}
