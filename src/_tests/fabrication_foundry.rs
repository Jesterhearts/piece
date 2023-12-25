use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{CardId, Database, InExile, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::{Phase, Turn},
};

#[test]
fn exile_return_to_battlefield() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("".to_string(), 20);
    all_players[player].infinite_mana();

    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Fabrication Foundry");
    let gy = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let exiled = CardId::upload(&mut db, &cards, player, "Abzan Banner");

    card.move_to_battlefield(&mut db);
    gy.move_to_graveyard(&mut db);
    exiled.move_to_battlefield(&mut db);

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player, card, 1);
    // Compute exile targets
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose exile card
    let result = results.resolve(&mut db, &mut all_players, Some(1));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay white
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay generic
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose gy target
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Complete
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve ability
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(player.get_cards::<OnBattlefield>(&mut db), [card, gy]);
    assert_eq!(player.get_cards::<InExile>(&mut db), [exiled]);

    Ok(())
}
