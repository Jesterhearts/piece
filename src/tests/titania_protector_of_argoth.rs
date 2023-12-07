use enumset::enum_set;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    card::Color,
    deck::Deck,
    effects::{Token, TokenCreature},
    in_play::{AllCards, AllModifiers},
    load_cards,
    player::Player,
    stack::Stack,
    types::{Subtype, Type},
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

    let results = battlefield.add(&mut all_cards, &mut modifiers, land, vec![]);
    assert_eq!(results, []);
    let results =
        battlefield.permanent_to_graveyard(&mut all_cards, &mut modifiers, &mut stack, land);
    assert_eq!(results, []);

    let titania = all_cards.add(&cards, player.clone(), "Titania, Protector of Argoth");
    let results = battlefield.add(&mut all_cards, &mut modifiers, titania, vec![]);
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
            count: 1,
            types: enum_set!(Type::Land | Type::BasicLand),
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

#[test]
fn graveyard_trigger() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();

    let player = Player::new_ref(Deck::empty());

    let land = all_cards.add(&cards, player.clone(), "Forest");

    let results = battlefield.add(&mut all_cards, &mut modifiers, land, vec![]);
    assert_eq!(results, []);

    let titania = all_cards.add(&cards, player.clone(), "Titania, Protector of Argoth");
    let results = battlefield.add(&mut all_cards, &mut modifiers, titania, vec![]);
    assert_eq!(
        results,
        [UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
            count: 1,
            types: enum_set!(Type::Land | Type::BasicLand),
            valid_targets: vec![]
        }]
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
        [UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
            count: 1,
            types: enum_set!(Type::Land | Type::BasicLand),
            valid_targets: vec![]
        }]
    );

    let results =
        battlefield.permanent_to_graveyard(&mut all_cards, &mut modifiers, &mut stack, land);
    assert_eq!(
        results,
        [UnresolvedActionResult::CreateToken {
            source: titania,
            token: Token::Creature(TokenCreature {
                name: "Elemental".to_owned(),
                types: enum_set!(Type::Creature),
                subtypes: enum_set!(Subtype::Elemental),
                colors: enum_set!(Color::Green),
                power: 5,
                toughness: 3,
            })
        }]
    );

    let results =
        battlefield.maybe_resolve(&mut all_cards, &mut modifiers, &mut stack, player, results);
    assert_eq!(results, []);

    assert_eq!(battlefield.permanents.len(), 2);

    Ok(())
}
