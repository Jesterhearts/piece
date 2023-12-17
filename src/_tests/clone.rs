use std::collections::VecDeque;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{
        ActionResult, Battlefield, PendingResult, PendingResults, ResolutionResult,
        UnresolvedAction, UnresolvedActionResult,
    },
    effects::Effect,
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::ActiveTarget,
};

#[test]
fn etb_clones() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack_or_hand(&mut db, creature, None);
    assert_eq!(results, PendingResults::default());

    let clone = CardId::upload(&mut db, &cards, player, "Clone");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, clone, None);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([
                PendingResult {
                    apply_immediately: vec![],
                    to_resolve: VecDeque::from([UnresolvedAction::new(
                        &mut db,
                        Some(clone),
                        UnresolvedActionResult::Effect(Effect::CopyOfAnyCreatureNonTargeting),
                        vec![vec![ActiveTarget::Battlefield { id: creature }]],
                        true,
                    )]),
                    then_apply: vec![],
                },
                PendingResult {
                    apply_immediately: vec![],
                    to_resolve: Default::default(),
                    then_apply: vec![ActionResult::AddToBattlefieldSkipReplacementEffects(
                        clone, None
                    )],
                }
            ]),
            applied: false,
        }
    );

    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(clone.cloning(&mut db), Some(creature.into()));

    Ok(())
}

#[test]
fn etb_no_targets_dies() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);

    let clone = CardId::upload(&mut db, &cards, player, "Clone");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, clone, None);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([
                PendingResult {
                    apply_immediately: vec![],
                    to_resolve: VecDeque::from([UnresolvedAction::new(
                        &mut db,
                        Some(clone),
                        UnresolvedActionResult::Effect(Effect::CopyOfAnyCreatureNonTargeting),
                        vec![vec![]],
                        true,
                    )]),
                    then_apply: vec![],
                },
                PendingResult {
                    apply_immediately: vec![],
                    to_resolve: Default::default(),
                    then_apply: vec![ActionResult::AddToBattlefieldSkipReplacementEffects(
                        clone, None
                    )],
                }
            ]),
            applied: false,
        }
    );

    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let results = Battlefield::check_sba(&mut db);
    assert_eq!(results, [ActionResult::PermanentToGraveyard(clone)]);

    Ok(())
}
