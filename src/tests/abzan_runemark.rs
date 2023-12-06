use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield},
    deck::Deck,
    in_play::{AllCards, AllModifiers},
    load_cards,
    player::Player,
    stack::Stack,
};

#[test]
fn aura_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());
    player.borrow_mut().infinite_mana();

    let creature = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, creature, vec![]);

    let aura = all_cards.add(&cards, player.clone(), "Abzan Runemark");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, aura, vec![creature]);

    let card = &all_cards[creature];
    assert_eq!(card.card.power(), Some(6));
    assert_eq!(card.card.toughness(), Some(4));

    let creature2 = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, creature2, vec![]);

    let card2 = &all_cards[creature2];
    assert_eq!(card2.card.power(), Some(4));
    assert_eq!(card2.card.toughness(), Some(2));

    let results =
        battlefield.permanent_to_graveyard(&mut all_cards, &mut modifiers, &mut stack, aura);
    assert_eq!(results, []);

    let card = &all_cards[creature];
    assert_eq!(card.card.power(), Some(4));
    assert_eq!(card.card.toughness(), Some(2));

    assert!(battlefield.no_modifiers());

    Ok(())
}

#[test]
fn aura_leaves_battlefield_enchanting_leaves_battlefield() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());
    player.borrow_mut().infinite_mana();

    let creature = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, creature, vec![]);

    let aura = all_cards.add(&cards, player.clone(), "Abzan Runemark");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, aura, vec![creature]);

    let results = battlefield.check_sba(&all_cards);
    assert_eq!(results, []);

    let card = &all_cards[creature];
    assert_eq!(card.card.power(), Some(6));
    assert_eq!(card.card.toughness(), Some(4));

    let results =
        battlefield.permanent_to_graveyard(&mut all_cards, &mut modifiers, &mut stack, creature);
    assert_eq!(results, []);
    let results = battlefield.check_sba(&all_cards);

    assert_eq!(results, [ActionResult::PermanentToGraveyard(aura)]);

    let results =
        battlefield.apply_action_results(&mut all_cards, &mut modifiers, &mut stack, results);
    assert_eq!(results, []);

    assert!(battlefield.no_modifiers());
    assert!(battlefield.is_empty());

    Ok(())
}
