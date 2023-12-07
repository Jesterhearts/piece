use std::collections::HashSet;

use enumset::{enum_set, EnumSet};
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    deck::Deck,
    effects::Destination,
    in_play::{AllCards, AllModifiers},
    load_cards,
    player::Player,
    targets::{Comparison, Restriction},
    types::Type,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut battlefield = Battlefield::default();

    let player = Player::new_ref(Deck::empty());

    let bear = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
    player.borrow_mut().deck.place_on_top(bear);

    let spell = all_cards.add(&cards, player.clone(), "Annul");
    player.borrow_mut().deck.place_on_top(spell);

    let elesh = all_cards.add(&cards, player.clone(), "Elesh Norn, Grand Cenobite");
    player.borrow_mut().deck.place_on_top(elesh);

    let recruiter = all_cards.add(&cards, player.clone(), "Recruiter of the Guard");
    let results = battlefield.add(&mut all_cards, &mut modifiers, recruiter, vec![]);
    assert_eq!(
        results,
        [UnresolvedActionResult::TutorLibrary {
            source: recruiter,
            destination: Destination::Hand,
            targets: HashSet::from([bear]),
            reveal: true,
            restrictions: HashSet::from([
                Restriction::Toughness(Comparison::LessThanOrEqual(2)),
                Restriction::OfType {
                    types: enum_set!(Type::Creature),
                    subtypes: enum_set!()
                }
            ])
        }]
    );

    Ok(())
}
