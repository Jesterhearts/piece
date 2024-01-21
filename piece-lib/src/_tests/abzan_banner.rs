use indexmap::IndexSet;
use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    in_play::Database,
    library::Library,
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    protogen::{
        ids::CardId,
        mana::ManaSource,
        mana::{Mana, ManaRestriction},
    },
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
    all_players[&player].infinite_mana();

    let mut db = Database::new(all_players);

    let land = CardId::upload(&mut db, &cards, player.clone(), "Forest");
    Library::place_on_top(&mut db, &player, land.clone());

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player.clone(), "Abzan Banner");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &card, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefields::activate_ability(&mut db, &None, &player, &card, 1);
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

    assert_eq!(db.graveyard[&player], IndexSet::from([card]));
    assert_eq!(db.hand[&player], IndexSet::from([land]));

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

    let card = CardId::upload(&mut db, &cards, player.clone(), "Abzan Banner");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &card, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefields::activate_ability(&mut db, &None, &player, &card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::PendingChoice);

    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        db.all_players[&player].mana_pool.all_mana().collect_vec(),
        [
            (1, Mana::WHITE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLUE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLACK, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::RED, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::GREEN, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::COLORLESS, ManaSource::ANY, ManaRestriction::NONE),
        ]
    );

    Ok(())
}
