use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield},
    deck::Deck,
    effects::{ActivatedAbilityEffect, GainMana},
    in_play::{AllCards, AllModifiers, EffectsInPlay},
    load_cards,
    mana::Mana,
    player::Player,
    stack::Stack,
};

#[test]
fn sacrifice_draw_card() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());
    player.borrow_mut().infinite_mana();

    let card = all_cards.add(&cards, player.clone(), "Abzan Banner");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, card);

    let card = battlefield.select_card(0);
    let result = battlefield.activate_ability(card, &all_cards, &stack, 1, None);
    assert_eq!(
        result,
        [
            ActionResult::TapPermanent(card),
            ActionResult::PermanentToGraveyard(card),
            ActionResult::AddToStack(
                EffectsInPlay {
                    effects: vec![ActivatedAbilityEffect::ControllerDrawCards(1)],
                    source: card,
                    controller: player
                },
                None
            )
        ]
    );

    Ok(())
}

#[test]
fn add_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());
    player.borrow_mut().infinite_mana();

    let card = all_cards.add(&cards, player.clone(), "Abzan Banner");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, card);

    let card = battlefield.select_card(0);
    let result = battlefield.activate_ability(card, &all_cards, &stack, 0, None);
    assert_eq!(
        result,
        [
            ActionResult::TapPermanent(card),
            ActionResult::AddToStack(
                EffectsInPlay {
                    effects: vec![ActivatedAbilityEffect::GainMana {
                        mana: GainMana::Choice {
                            choices: vec![vec![Mana::White], vec![Mana::Black], vec![Mana::Green]],
                        }
                    }],
                    source: card,
                    controller: player
                },
                None
            )
        ]
    );

    Ok(())
}
