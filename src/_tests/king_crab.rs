use std::collections::{HashSet, VecDeque};

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    turns::{Phase, Turn},
};

#[test]
fn place_on_top() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "King Crab");
    card.move_to_battlefield(&mut db);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    creature.move_to_battlefield(&mut db);

    assert_eq!(
        card.valid_targets(&mut db, &HashSet::default()),
        vec![vec![ActiveTarget::Battlefield { id: creature }]]
    );

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 0);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // pay costs
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    //end pay costs
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(all_players[player].deck.cards, VecDeque::from([creature]));

    Ok(())
}
