use std::collections::HashSet;

use enumset::enum_set;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    controller::Controller,
    deck::Deck,
    in_play::{AllCards, AllModifiers},
    load_cards,
    player::Player,
    stack::Stack,
    types::Type,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();

    let player = Player::new_ref(Deck::empty());

    let land = all_cards.add(&cards, player.clone(), "Forest");
    let nonland = all_cards.add(&cards, player.clone(), "Annul");

    player.borrow_mut().deck.place_on_top(land);
    player.borrow_mut().deck.place_on_top(nonland);

    let glowspore = all_cards.add(&cards, player.clone(), "Glowspore Shaman");
    let results = battlefield.add(&mut all_cards, &mut modifiers, glowspore, vec![]);
    assert_eq!(
        results,
        [
            UnresolvedActionResult::Mill {
                count: 3,
                valid_targets: HashSet::from([player.clone()])
            },
            UnresolvedActionResult::ReturnFromGraveyardToLibrary {
                count: 1,
                controller: Controller::You,
                types: enum_set!(Type::BasicLand | Type::Land),
                valid_targets: vec![]
            }
        ]
    );

    let results = battlefield.maybe_resolve(
        &mut all_cards,
        &mut modifiers,
        &mut stack,
        player.clone(),
        results,
    );

    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToLibrary {
            count: 1,
            controller: Controller::You,
            types: enum_set!(Type::BasicLand | Type::Land),
            valid_targets: vec![land]
        }]
    );

    let results = battlefield.maybe_resolve(
        &mut all_cards,
        &mut modifiers,
        &mut stack,
        player.clone(),
        results,
    );

    assert_eq!(results, []);

    Ok(())
}
