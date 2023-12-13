use std::collections::{HashSet, VecDeque};

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{
        Battlefield, PendingResult, PendingResults, ResolutionResult, UnresolvedAction,
        UnresolvedActionResult,
    },
    controller::ControllerRestriction,
    effects::{Effect, Mill, ReturnFromGraveyardToLibrary},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::ActiveTarget,
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
    let nonland = CardId::upload(&mut db, &cards, player, "Annul");

    all_players[player].deck.place_on_top(&mut db, land);
    all_players[player].deck.place_on_top(&mut db, nonland);

    let glowspore = CardId::upload(&mut db, &cards, player, "Glowspore Shaman");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, glowspore, vec![]);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([PendingResult {
                apply_immediately: vec![],
                then_resolve: VecDeque::from([
                    UnresolvedAction {
                        source: glowspore,
                        result: UnresolvedActionResult::Effect(Effect::Mill(Mill {
                            count: 3,
                            target: ControllerRestriction::You,
                        })),
                        valid_targets: vec![ActiveTarget::Player { id: player }],
                        choices: vec![],
                        optional: false,
                    },
                    UnresolvedAction {
                        source: glowspore,
                        result: UnresolvedActionResult::Effect(
                            Effect::ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary {
                                count: 1,
                                controller: ControllerRestriction::You,
                                types: HashSet::from([Type::Land, Type::BasicLand]),
                            })
                        ),
                        valid_targets: vec![],
                        optional: false,
                        choices: vec![]
                    }
                ]),
                recompute: true
            },])
        },
    );

    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([PendingResult {
                apply_immediately: vec![],
                then_resolve: VecDeque::from([UnresolvedAction {
                    source: glowspore,
                    result: UnresolvedActionResult::Effect(Effect::ReturnFromGraveyardToLibrary(
                        ReturnFromGraveyardToLibrary {
                            count: 1,
                            controller: ControllerRestriction::You,
                            types: HashSet::from([Type::Land, Type::BasicLand]),
                        }
                    )),
                    valid_targets: vec![ActiveTarget::Graveyard { id: land }],
                    optional: false,
                    choices: vec![]
                }]),
                recompute: true
            },])
        },
    );

    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    Ok(())
}
