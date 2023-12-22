use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    turns::{Phase, Turn},
};

#[test]
fn sacrifice_gain_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].mana_pool.colorless_mana += 1;
    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let attendant = CardId::upload(&mut db, &cards, player, "Darigaaz's Attendant");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, attendant, None);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player, attendant, 0);

    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(all_players[player].mana_pool.black_mana, 1);
    assert_eq!(all_players[player].mana_pool.red_mana, 1);
    assert_eq!(all_players[player].mana_pool.green_mana, 1);

    Ok(())
}
