use std::collections::VecDeque;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{
        ActionResult, Battlefield, PendingResult, PendingResults, ResolutionResult,
        UnresolvedAction, UnresolvedActionResult,
    },
    in_play::{CardId, Database, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
};

#[test]
fn enters_tapped() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_owned(), 20);

    let card = CardId::upload(&mut db, &cards, player, "Krosan Verge");
    let results = Battlefield::add_from_stack_or_hand(&mut db, card, vec![]);
    assert_eq!(results, PendingResults::default());

    assert!(card.tapped(&mut db));

    Ok(())
}

#[test]
fn tutors() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_owned(), 20);
    all_players[player].infinite_mana();

    let forest = CardId::upload(&mut db, &cards, player, "Forest");
    all_players[player].deck.place_on_top(&mut db, forest);

    let plains = CardId::upload(&mut db, &cards, player, "Plains");
    all_players[player].deck.place_on_top(&mut db, plains);

    let annul = CardId::upload(&mut db, &cards, player, "Annul");
    all_players[player].deck.place_on_top(&mut db, annul);

    let card = CardId::upload(&mut db, &cards, player, "Krosan Verge");
    let results = Battlefield::add_from_stack_or_hand(&mut db, card, vec![]);
    assert_eq!(results, PendingResults::default());

    card.untap(&mut db);

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, card, 0);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([PendingResult {
                apply_immediately: vec![
                    ActionResult::TapPermanent(card),
                    ActionResult::PermanentToGraveyard(card)
                ],
                then_resolve: VecDeque::from([UnresolvedAction {
                    source: Some(card),
                    result: UnresolvedActionResult::Ability(
                        *card.activated_abilities(&mut db).first().unwrap()
                    ),
                    valid_targets: vec![
                        ActiveTarget::Library { id: forest },
                        ActiveTarget::Library { id: plains }
                    ],
                    choices: Default::default(),
                    optional: false
                },]),
                recompute: true
            },])
        }
    );

    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert!(forest.is_in_location::<OnBattlefield>(&db));
    assert!(plains.is_in_location::<OnBattlefield>(&db));

    Ok(())
}
