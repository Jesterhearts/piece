use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, UnresolvedActionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
};

#[test]
fn etb_clones() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let results = Battlefield::add_from_stack(&mut db, creature, vec![]);
    assert_eq!(results, []);

    let clone = CardId::upload(&mut db, &cards, player, "Clone");
    let results = Battlefield::add_from_stack(&mut db, clone, vec![]);
    assert_eq!(
        results,
        [UnresolvedActionResult::CloneCreatureNonTargeting {
            source: clone,
            valid_targets: vec![creature]
        }]
    );

    let results = results
        .into_iter()
        .map(|result| match result {
            UnresolvedActionResult::CloneCreatureNonTargeting {
                source,
                mut valid_targets,
            } => ActionResult::CloneCreatureNonTargeting {
                source,
                target: valid_targets.pop(),
            },
            _ => unreachable!(),
        })
        .collect();

    let results = Battlefield::apply_action_results(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    assert_eq!(clone.cloning(&mut db), Some(creature.into()));

    Ok(())
}

#[test]
fn etb_no_targets_dies() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);

    let clone = CardId::upload(&mut db, &cards, player, "Clone");
    let results = Battlefield::add_from_stack(&mut db, clone, vec![]);
    assert_eq!(
        results,
        [UnresolvedActionResult::CloneCreatureNonTargeting {
            source: clone,
            valid_targets: vec![]
        }]
    );

    let results = results
        .into_iter()
        .map(|result| match result {
            UnresolvedActionResult::CloneCreatureNonTargeting {
                source,
                mut valid_targets,
            } => ActionResult::CloneCreatureNonTargeting {
                source,
                target: valid_targets.pop(),
            },
            _ => unreachable!(),
        })
        .collect();

    let results = Battlefield::apply_action_results(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    let results = Battlefield::check_sba(&mut db);
    assert_eq!(results, [ActionResult::PermanentToGraveyard(clone)]);

    Ok(())
}
