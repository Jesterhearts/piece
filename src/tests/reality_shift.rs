use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    deck::{Deck, DeckDefinition},
    in_play::{AllCards, AllModifiers},
    load_cards,
    player::Player,
    stack::{ActiveTarget, Stack, StackResult},
    types::Subtype,
};

#[test]
fn resolves_shift() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let player1 = Player::new_ref(Deck::empty());
    let player2 = Player::new_ref(Deck::empty());
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut battlefield = Battlefield::default();
    let mut stack = Stack::default();

    let mut deck = DeckDefinition::default();
    deck.add_card("Annul".to_owned(), 1);
    all_cards.build_deck_for_player(&cards, &deck, player1.clone());

    let creature = all_cards.add(&cards, player1.clone(), "Alpine Grizzly");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, creature);

    let shift = all_cards.add(&cards, player2.clone(), "Reality Shift");

    stack.push_card(
        &all_cards,
        shift,
        Some(ActiveTarget::Battlefield { id: creature }),
        None,
    );

    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(
        results,
        [
            StackResult::ExileTarget(creature),
            StackResult::ManifestTopOfLibrary(player1.clone())
        ]
    );

    stack.apply_results(&mut all_cards, &mut modifiers, &mut battlefield, results);

    let creature = all_cards.add(&cards, player1.clone(), "Alpine Grizzly");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, creature);

    assert_eq!(
        all_cards[creature].card.subtypes(),
        HashSet::from([Subtype::Bear])
    );
    assert_eq!(all_cards[creature].card.power(), 4);
    assert_eq!(all_cards[creature].card.toughness(), 2);

    let creature = battlefield.select_card(0);

    assert_eq!(all_cards[creature].card.subtypes(), Default::default());
    assert_eq!(all_cards[creature].card.power(), 2);
    assert_eq!(all_cards[creature].card.toughness(), 2);
    assert!(all_cards[creature].face_down);
    assert!(all_cards[creature].manifested);

    Ok(())
}
