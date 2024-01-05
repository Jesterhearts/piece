use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::CardId,
    in_play::{self, Database, OnBattlefield},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::Stack,
    turns::{Phase, Turn},
    types::Subtype,
};

#[test]
fn modify_base_p_t_works() -> anyhow::Result<()> {
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

    let card = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, &None, player, card, 0);

    // Pay costs
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // end pay costs
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(card.power(&mut db), Some(5));
    assert_eq!(card.toughness(&mut db), Some(5));
    assert_eq!(
        card.subtypes(&db),
        IndexSet::from([Subtype::Elf, Subtype::Shaman, Subtype::Dinosaur])
    );

    let mut results = Battlefield::end_turn(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(card.power(&mut db), Some(1));
    assert_eq!(card.toughness(&mut db), Some(1));
    assert_eq!(
        card.subtypes(&db),
        IndexSet::from([Subtype::Elf, Subtype::Shaman])
    );

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
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
    let turn = Turn::new(&mut db, &all_players);

    let card = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    card.move_to_stack(&mut db, vec![], None, vec![]);
    let targets = vec![vec![Stack::target_nth(&mut db, 0)]];
    counterspell.move_to_stack(&mut db, targets, None, vec![]);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(Stack::in_stack(&mut db).len(), 1);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), [card]);

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_green_uncounterable() -> anyhow::Result<()> {
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
    let turn = Turn::new(&mut db, &all_players);

    let card1 = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card1, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    card2.move_to_stack(&mut db, vec![], None, vec![]);
    let targets = vec![vec![Stack::target_nth(&mut db, 0)]];
    counterspell.move_to_stack(&mut db, targets, None, vec![]);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(Stack::in_stack(&mut db).len(), 1);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), [card1, card2]);

    Ok(())
}

#[test]
fn resolves_counterspells_respecting_green_uncounterable_other_player() -> anyhow::Result<()> {
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
    let player2 = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    let card1 = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let card2 = CardId::upload(&mut db, &cards, player2, "Alpine Grizzly");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card1, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    card2.move_to_stack(&mut db, vec![], None, vec![]);
    let targets = vec![vec![Stack::target_nth(&mut db, 0)]];
    counterspell.move_to_stack(&mut db, targets, None, vec![]);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Stack::is_empty(&mut db));
    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), [card1]);

    Ok(())
}
