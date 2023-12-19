use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::{Phase, Turn},
};

#[test]
fn adds_ability() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let equipment = CardId::upload(&mut db, &cards, player, "Paradise Mantle");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, equipment, None);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, None);

    assert_eq!(creature.activated_abilities(&db), []);

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, equipment, 0);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.activated_abilities(&db).len(), 1);

    Ok(())
}
