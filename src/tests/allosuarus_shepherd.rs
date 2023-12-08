use std::collections::HashSet;

use enumset::{enum_set, EnumSet};
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    controller::Controller,
    deck::Deck,
    effects::{ActivatedAbilityEffect, BattlefieldModifier, EffectDuration, ModifyBattlefield},
    in_play::{AllCards, AllModifiers, EffectsInPlay, ModifierInPlay, ModifierType},
    load_cards,
    player::Player,
    stack::{Stack, StackResult},
    targets::Restriction,
    types::Subtype,
};

#[test]
fn modify_base_p_t_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());
    player.borrow_mut().infinite_mana();

    let card = all_cards.add(&cards, player.clone(), "Allosaurus Shepherd");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, card, vec![]);

    let card = battlefield.select_card(0);
    let results = battlefield.activate_ability(card, &all_cards, &stack, 0);

    assert_eq!(
        results,
        [UnresolvedActionResult::AddToStack {
            card,
            effects: EffectsInPlay {
                effects: vec![ActivatedAbilityEffect::BattlefieldModifier(
                    BattlefieldModifier {
                        modifier: ModifyBattlefield {
                            base_power: Some(5),
                            base_toughness: Some(5),
                            add_subtypes: enum_set!(Subtype::Dinosaur),
                            ..Default::default()
                        },
                        controller: Controller::You,
                        duration: EffectDuration::UntilEndOfTurn,
                        restrictions: HashSet::from([Restriction::OfType {
                            types: enum_set!(),
                            subtypes: enum_set!(Subtype::Elf)
                        }]),
                    }
                ),],
                source: card,
                controller: player.clone(),
            },
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
    assert_eq!(results, []);

    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(
        results,
        [StackResult::ApplyToBattlefield {
            modifier: ModifierInPlay {
                source: card,
                modifier: BattlefieldModifier {
                    modifier: ModifyBattlefield {
                        base_power: Some(5),
                        base_toughness: Some(5),
                        add_subtypes: enum_set!(Subtype::Dinosaur),
                        ..Default::default()
                    },
                    controller: Controller::You,
                    duration: EffectDuration::UntilEndOfTurn,
                    restrictions: HashSet::from([Restriction::OfType {
                        types: enum_set!(),
                        subtypes: enum_set!(Subtype::Elf)
                    }]),
                },
                controller: player.clone(),
                modifying: Default::default(),
                modifier_type: ModifierType::Global,
            },
        },]
    );

    let results = stack.apply_results(&mut all_cards, &mut modifiers, &mut battlefield, results);
    assert_eq!(results, []);

    let card = battlefield.select_card(0);
    let card = &all_cards[card];
    assert_eq!(card.card.power(), Some(5));
    assert_eq!(card.card.toughness(), Some(5));

    assert_eq!(
        card.card.subtypes(),
        enum_set![Subtype::Elf | Subtype::Shaman | Subtype::Dinosaur]
    );

    battlefield.end_turn(&mut all_cards, &mut modifiers);

    let card = battlefield.select_card(0);
    let card = &all_cards[card];
    assert_eq!(card.card.power(), Some(1));
    assert_eq!(card.card.toughness(), Some(1));

    assert_eq!(
        card.card.subtypes(),
        enum_set![Subtype::Elf | Subtype::Shaman]
    );

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let player = Player::new_ref(Deck::empty());
    let mut all_cards = AllCards::default();
    let battlefield = Battlefield::default();
    let mut stack = Stack::default();

    let creature = all_cards.add(&cards, player.clone(), "Allosaurus Shepherd");
    let counterspell = all_cards.add(&cards, player.clone(), "Counterspell");

    stack.push_card(&all_cards, creature, None, None);
    stack.push_card(&all_cards, counterspell, stack.target_nth(0), None);

    assert_eq!(stack.stack.len(), 2);

    let result = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(result, []);

    assert_eq!(stack.stack.len(), 1);

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_green_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let player = Player::new_ref(Deck::empty());
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut battlefield = Battlefield::default();
    let mut stack = Stack::default();

    let creature_1 = all_cards.add(&cards, player.clone(), "Allosaurus Shepherd");
    let creature_2 = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
    let counterspell = all_cards.add(&cards, player.clone(), "Counterspell");

    stack.push_card(&all_cards, creature_1, None, None);
    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(results, [StackResult::AddToBattlefield(creature_1)]);

    let results = stack.apply_results(&mut all_cards, &mut modifiers, &mut battlefield, results);
    assert_eq!(results, []);

    stack.push_card(&all_cards, creature_2, None, None);
    stack.push_card(&all_cards, counterspell, stack.target_nth(0), None);

    assert_eq!(stack.stack.len(), 2);

    let result = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(result, []);

    assert_eq!(stack.stack.len(), 1);

    let result = stack.resolve_1(&all_cards, &battlefield);
    assert!(stack.is_empty());
    assert_eq!(result, [StackResult::AddToBattlefield(creature_2)]);

    Ok(())
}

#[test]
fn resolves_counterspells_respecting_green_uncounterable_other_player() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let player1 = Player::new_ref(Deck::empty());
    let player2 = Player::new_ref(Deck::empty());

    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut battlefield = Battlefield::default();
    let mut stack = Stack::default();

    let creature_1 = all_cards.add(&cards, player1.clone(), "Allosaurus Shepherd");
    let creature_2 = all_cards.add(&cards, player2.clone(), "Alpine Grizzly");
    let counterspell = all_cards.add(&cards, player1.clone(), "Counterspell");

    stack.push_card(&all_cards, creature_1, None, None);
    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(results, [StackResult::AddToBattlefield(creature_1)]);
    let results = stack.apply_results(&mut all_cards, &mut modifiers, &mut battlefield, results);
    assert_eq!(results, []);

    let countered = stack.push_card(&all_cards, creature_2, None, None);
    stack.push_card(&all_cards, counterspell, stack.target_nth(0), None);

    assert_eq!(stack.stack.len(), 2);

    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(results, [StackResult::SpellCountered { id: countered }]);

    let results = stack.apply_results(&mut all_cards, &mut modifiers, &mut battlefield, results);
    assert_eq!(results, []);

    Ok(())
}
