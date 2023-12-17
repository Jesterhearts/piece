use crate::{
    battlefield::{Battlefield, PendingResults, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::{Phase, Turn},
};

#[test]
fn plains() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Plains");

    let results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    assert_eq!(results, PendingResults::default());
    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 0);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Stack::is_empty(&mut db));

    assert_eq!(all_players[player].mana_pool.white_mana, 1);

    Ok(())
}

#[test]
fn island() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);

    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Island");

    let results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    assert_eq!(results, PendingResults::default());
    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 0);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Stack::is_empty(&mut db));
    assert_eq!(all_players[player].mana_pool.blue_mana, 1);

    Ok(())
}

#[test]
fn swamp() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Swamp");

    let results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    assert_eq!(results, PendingResults::default());
    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 0);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Stack::is_empty(&mut db));

    assert_eq!(all_players[player].mana_pool.black_mana, 1);

    Ok(())
}

#[test]
fn mountain() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Mountain");

    let results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    assert_eq!(results, PendingResults::default());
    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 0);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Stack::is_empty(&mut db));
    assert_eq!(all_players[player].mana_pool.red_mana, 1);

    Ok(())
}

#[test]
fn forest() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Forest");

    let results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    assert_eq!(results, PendingResults::default());
    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 0);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Stack::is_empty(&mut db));
    assert_eq!(all_players[player].mana_pool.green_mana, 1);

    Ok(())
}
