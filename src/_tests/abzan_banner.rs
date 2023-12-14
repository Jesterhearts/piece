use std::collections::VecDeque;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{
        ActionResult, Battlefield, PendingResult, PendingResults, ResolutionResult,
        UnresolvedAction, UnresolvedActionResult,
    },
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
};

#[test]
fn sacrifice_draw_card() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let results = Battlefield::add_from_stack_or_hand(&mut db, card, vec![]);
    assert_eq!(results, PendingResults::default());

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, card, 0);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([PendingResult {
                apply_immediately: vec![
                    ActionResult::TapPermanent(card),
                    ActionResult::PermanentToGraveyard(card),
                ],
                then_resolve: VecDeque::from([UnresolvedAction {
                    source: card,
                    result: UnresolvedActionResult::Ability(
                        card.activated_abilities(&mut db).first().copied().unwrap()
                    ),
                    valid_targets: vec![],
                    choices: vec![],
                    optional: false,
                }]),
                recompute: true
            }])
        }
    );

    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    Ok(())
}

#[test]
fn add_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);

    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let results = Battlefield::add_from_stack_or_hand(&mut db, card, vec![]);
    assert_eq!(results, PendingResults::default());

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, card, 1);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::PendingChoice);

    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(all_players[player].mana_pool.white_mana, 1);

    Ok(())
}
