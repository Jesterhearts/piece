use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
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
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());
    player.borrow_mut().infinite_mana();

    let card = all_cards.add(&cards, player.clone(), "Darigaaz's Attendant");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, card, vec![]);

    let card = battlefield.select_card(0);
    let results = battlefield.activate_ability(card, &all_cards, &stack, 0);
    assert_eq!(
        results,
        [
            UnresolvedActionResult::PermanentToGraveyard(card),
            UnresolvedActionResult::AddToStack {
                card,
                effects: EffectsInPlay {
                    effects: vec![ActivatedAbilityEffect::GainMana {
                        mana: GainMana::Specific {
                            gains: vec![Mana::Black, Mana::Red, Mana::Green]
                        }
                    }],
                    source: card,
                    controller: player.clone(),
                },
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
    assert_eq!(results, []);

    Ok(())
}
