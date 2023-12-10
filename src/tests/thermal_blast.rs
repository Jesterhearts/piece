use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield},
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
    stack::{ActiveTarget, Stack, StackResult},
};

#[test]
fn damages_target() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let bear = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    bear.move_to_battlefield(&db)?;

    let blast = CardId::upload(&db, &cards, player, "Thermal Blast")?;
    blast.move_to_stack(&db, HashSet::from([ActiveTarget::Battlefield { id: bear }]))?;

    let results = Stack::resolve_1(&db)?;
    assert_eq!(
        results,
        [
            StackResult::DamageTarget {
                quantity: 3,
                target: bear
            },
            StackResult::StackToGraveyard(blast)
        ]
    );

    Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(bear.marked_damage(&db)?, 3);

    let results = Battlefield::check_sba(&db)?;
    assert_eq!(results, [ActionResult::PermanentToGraveyard(bear)]);
    let results = Battlefield::apply_action_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);
    assert_eq!(Battlefield::creatures(&db)?, []);

    Ok(())
}

#[test]
fn damages_target_threshold() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    for _ in 0..7 {
        let card = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
        card.move_to_graveyard(&db)?;
    }

    let bear = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    bear.move_to_battlefield(&db)?;

    let blast = CardId::upload(&db, &cards, player, "Thermal Blast")?;
    blast.move_to_stack(&db, HashSet::from([ActiveTarget::Battlefield { id: bear }]))?;

    let results = Stack::resolve_1(&db)?;
    assert_eq!(
        results,
        [
            StackResult::DamageTarget {
                quantity: 5,
                target: bear
            },
            StackResult::StackToGraveyard(blast)
        ]
    );

    Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(bear.marked_damage(&db)?, 5);

    let results = Battlefield::check_sba(&db)?;
    assert_eq!(results, [ActionResult::PermanentToGraveyard(bear)]);
    let results = Battlefield::apply_action_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);
    assert_eq!(Battlefield::creatures(&db)?, []);

    Ok(())
}

#[test]
fn damages_target_threshold_other_player() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let other = all_players.new_player();

    for _ in 0..7 {
        let card = CardId::upload(&db, &cards, other, "Alpine Grizzly")?;
        card.move_to_graveyard(&db)?;
    }

    let bear = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    bear.move_to_battlefield(&db)?;

    let blast = CardId::upload(&db, &cards, player, "Thermal Blast")?;
    blast.move_to_stack(&db, HashSet::from([ActiveTarget::Battlefield { id: bear }]))?;

    let results = Stack::resolve_1(&db)?;
    assert_eq!(
        results,
        [
            StackResult::DamageTarget {
                quantity: 3,
                target: bear
            },
            StackResult::StackToGraveyard(blast)
        ]
    );

    Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(bear.marked_damage(&db)?, 3);

    let results = Battlefield::check_sba(&db)?;
    assert_eq!(results, [ActionResult::PermanentToGraveyard(bear)]);
    let results = Battlefield::apply_action_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);
    assert_eq!(Battlefield::creatures(&db)?, []);

    Ok(())
}
