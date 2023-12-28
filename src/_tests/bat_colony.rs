use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{self, CardId, Database, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::{Phase, Turn},
    types::Type,
};

#[test]
fn spawns_bats() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();

    let player = all_players.new_player(String::default(), 20);

    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let cave1 = CardId::upload(&mut db, &cards, player, "Hidden Courtyard");
    cave1.move_to_battlefield(&mut db);
    let cave2 = CardId::upload(&mut db, &cards, player, "Hidden Courtyard");
    cave2.move_to_battlefield(&mut db);
    let cave3 = CardId::upload(&mut db, &cards, player, "Hidden Courtyard");
    cave3.move_to_battlefield(&mut db);

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player, cave1, 0);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player, cave2, 0);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player, cave3, 0);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let bat_colony = CardId::upload(&mut db, &cards, player, "Bat Colony");
    let mut results = Stack::move_card_to_stack_from_hand(&mut db, bat_colony, true);
    // Pay white
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    // Pay generic
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Cast card
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve card
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve etb
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Should have 3 bats
    assert_eq!(
        in_play::cards::<OnBattlefield>(&mut db)
            .into_iter()
            .filter(|card| card.types_intersect(&db, &IndexSet::from([Type::Creature])))
            .count(),
        3
    );

    Ok(())
}
