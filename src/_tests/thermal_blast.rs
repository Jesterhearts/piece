use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, PendingResults, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
};

#[test]
fn damages_target() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player, "Thermal Blast");
    blast.move_to_stack(&mut db, vec![ActiveTarget::Battlefield { id: bear }]);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [
            ActionResult::DamageTarget {
                quantity: 3,
                target: ActiveTarget::Battlefield { id: bear }
            },
            ActionResult::StackToGraveyard(blast)
        ]
    );

    let mut results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(bear.marked_damage(&mut db), 3);

    let results = Battlefield::check_sba(&mut db);
    assert_eq!(results, [ActionResult::PermanentToGraveyard(bear)]);
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());
    assert_eq!(Battlefield::creatures(&mut db), []);

    Ok(())
}

#[test]
fn damages_target_threshold() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    for _ in 0..7 {
        let card = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
        card.move_to_graveyard(&mut db);
    }

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player, "Thermal Blast");
    blast.move_to_stack(&mut db, vec![ActiveTarget::Battlefield { id: bear }]);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [
            ActionResult::DamageTarget {
                quantity: 5,
                target: ActiveTarget::Battlefield { id: bear }
            },
            ActionResult::StackToGraveyard(blast)
        ]
    );

    let mut results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(bear.marked_damage(&mut db), 5);

    let results = Battlefield::check_sba(&mut db);
    assert_eq!(results, [ActionResult::PermanentToGraveyard(bear)]);
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());
    assert_eq!(Battlefield::creatures(&mut db), []);

    Ok(())
}

#[test]
fn damages_target_threshold_other_player() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let other = all_players.new_player(20);

    for _ in 0..7 {
        let card = CardId::upload(&mut db, &cards, other, "Alpine Grizzly");
        card.move_to_graveyard(&mut db);
    }

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let blast = CardId::upload(&mut db, &cards, player, "Thermal Blast");
    blast.move_to_stack(&mut db, vec![ActiveTarget::Battlefield { id: bear }]);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [
            ActionResult::DamageTarget {
                quantity: 3,
                target: ActiveTarget::Battlefield { id: bear }
            },
            ActionResult::StackToGraveyard(blast)
        ]
    );

    let mut results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(bear.marked_damage(&mut db), 3);

    let results = Battlefield::check_sba(&mut db);
    assert_eq!(results, [ActionResult::PermanentToGraveyard(bear)]);
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());
    assert_eq!(Battlefield::creatures(&mut db), []);

    Ok(())
}
