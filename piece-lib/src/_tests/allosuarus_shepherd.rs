use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    effects::{PendingEffects, SelectionResult},
    in_play::Database,
    in_play::{CardId, CastFrom},
    load_cards,
    player::AllPlayers,
    protogen::types::Subtype,
    stack::Stack,
    turns::Phase,
    types::SubtypeSet,
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

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);

    let mut db = Database::new(all_players);
    db.all_players[player].infinite_mana();

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    card.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 0);
    // Pay costs
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::PendingChoice);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::PendingChoice);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // end pay costs
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(card.power(&db), Some(5));
    assert_eq!(card.toughness(&db), Some(5));
    assert_eq!(
        db[card].modified_subtypes,
        SubtypeSet::from([Subtype::ELF, Subtype::SHAMAN, Subtype::DINOSAUR])
    );

    let mut results = Battlefields::end_turn(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(card.power(&db), Some(1));
    assert_eq!(card.toughness(&db), Some(1));
    assert_eq!(
        db[card].modified_subtypes,
        SubtypeSet::from([Subtype::ELF, Subtype::SHAMAN])
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

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let card = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    let mut results = PendingEffects::default();
    results.apply_results(card.move_to_stack(&mut db, vec![], CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    let targets = vec![db.stack.target_nth(0)];
    results.apply_results(counterspell.move_to_stack(&mut db, targets, CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(db.stack.entries.len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(db.stack.entries.len(), 1);

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

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let card1 = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    card1.move_to_battlefield(&mut db);

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    let mut results = PendingEffects::default();
    results.apply_results(card2.move_to_stack(&mut db, vec![], CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let targets = vec![db.stack.target_nth(0)];
    results.apply_results(counterspell.move_to_stack(&mut db, targets, CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(db.stack.entries.len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(db.stack.entries.len(), 1);

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
        [card1, card2]
    );

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

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let player2 = all_players.new_player("Player".to_string(), 20);

    let mut db = Database::new(all_players);

    let card1 = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    card1.move_to_battlefield(&mut db);

    let card2 = CardId::upload(&mut db, &cards, player2, "Alpine Grizzly");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    let mut results = PendingEffects::default();
    results.apply_results(card2.move_to_stack(&mut db, vec![], CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let targets = vec![db.stack.target_nth(0)];
    results.apply_results(counterspell.move_to_stack(&mut db, targets, CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(db.stack.entries.len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(db.stack.is_empty());
    assert_eq!(
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
            .collect_vec(),
        [card1]
    );

    Ok(())
}
