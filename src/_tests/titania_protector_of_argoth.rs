use std::collections::{HashSet, VecDeque};

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{
        ActionResult, Battlefield, PendingResult, PendingResults, ResolutionResult,
        UnresolvedAction, UnresolvedActionResult,
    },
    effects::{Effect, ReturnFromGraveyardToBattlefield},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    types::Type,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_graveyard(&mut db);

    let titania = CardId::upload(&mut db, &cards, player, "Titania, Protector of Argoth");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, titania, vec![]);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([PendingResult {
                apply_immediately: vec![],
                then_resolve: VecDeque::from([UnresolvedAction {
                    source: titania,
                    result: UnresolvedActionResult::Effect(
                        Effect::ReturnFromGraveyardToBattlefield(
                            ReturnFromGraveyardToBattlefield {
                                count: 1,
                                types: HashSet::from([Type::Land, Type::BasicLand]),
                            }
                        )
                    ),
                    valid_targets: vec![ActiveTarget::Graveyard { id: land }],
                    choices: vec![],
                    optional: false,
                }]),
                recompute: false
            }])
        },
    );

    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    Ok(())
}

#[test]
fn graveyard_trigger() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_battlefield(&mut db);

    let titania = CardId::upload(&mut db, &cards, player, "Titania, Protector of Argoth");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, titania, vec![]);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::permanent_to_graveyard(&mut db, land);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let results = Stack::resolve_1(&mut db);
    assert!(matches!(
        results.as_slice(),
        [ActionResult::CreateToken { .. }]
    ));
    let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
    assert_eq!(results, PendingResults::default());

    assert_eq!(Battlefield::creatures(&mut db).len(), 2);

    Ok(())
}