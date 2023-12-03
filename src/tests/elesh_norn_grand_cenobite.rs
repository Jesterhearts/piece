use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    deck::Deck,
    in_play::{AllCards, AllModifiers},
    load_cards,
    player::Player,
    stack::Stack,
};

#[test]
fn modifies_battlefield() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());

    let elesh = all_cards.add(&cards, player.clone(), "Elesh Norn, Grand Cenobite");
    let bear = all_cards.add(&cards, player.clone(), "Alpine Grizzly");

    let results = battlefield.add(&mut all_cards, &mut modifiers, elesh);
    battlefield.apply_action_results(&mut all_cards, &mut modifiers, &mut stack, results);

    let _ = battlefield.add(&mut all_cards, &mut modifiers, bear);

    let card = &all_cards[elesh].card;
    assert_eq!(card.power(), 4);
    assert_eq!(card.toughness(), 7);

    let card = &all_cards[bear].card;
    assert_eq!(card.power(), 6);
    assert_eq!(card.toughness(), 4);

    battlefield.permanent_to_graveyard(&mut all_cards, &mut modifiers, &mut stack, elesh);

    let card = &all_cards[bear].card;
    assert_eq!(card.power(), 4);
    assert_eq!(card.toughness(), 2);

    Ok(())
}