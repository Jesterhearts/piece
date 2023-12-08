use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, UnresolvedActionResult},
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
};

#[test]
fn etb_clones() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let creature = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let results = Battlefield::add(&db, creature, vec![])?;
    assert_eq!(results, []);

    let clone = CardId::upload(&db, &cards, player, "Clone")?;
    let results = Battlefield::add(&db, clone, vec![])?;
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

    let results = Battlefield::apply_action_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    assert_eq!(clone.cloning(&db)?, Some(creature));

    Ok(())
}

#[test]
fn etb_no_targets_dies() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let clone = CardId::upload(&db, &cards, player, "Clone")?;
    let results = Battlefield::add(&db, clone, vec![])?;
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

    let results = Battlefield::apply_action_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    let results = Battlefield::check_sba(&db)?;
    assert_eq!(results, [ActionResult::PermanentToGraveyard(clone)]);

    Ok(())
}
