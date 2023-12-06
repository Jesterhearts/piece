use enumset::enum_set;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield},
    controller::Controller,
    deck::Deck,
    effects::{
        ActivatedAbilityEffect, AddPowerToughness, BattlefieldModifier, EffectDuration,
        ModifyBattlefield,
    },
    in_play::{AllCards, AllModifiers, EffectsInPlay, ModifierInPlay},
    load_cards,
    player::Player,
    stack::{ActiveTarget, Stack, StackResult},
    targets::Restriction,
};

#[test]
fn equipment_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());
    player.borrow_mut().infinite_mana();

    let equipment = all_cards.add(&cards, player.clone(), "+2 Mace");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, equipment);

    let creature = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, creature);

    let equipment = battlefield.select_card(0);
    let results = battlefield.activate_ability(
        equipment,
        &all_cards,
        &stack,
        0,
        Some(ActiveTarget::Battlefield { id: creature }),
    );

    assert_eq!(
        results,
        [ActionResult::AddToStack(
            equipment,
            EffectsInPlay {
                effects: vec![ActivatedAbilityEffect::Equip(vec![
                    ModifyBattlefield::AddPowerToughness(AddPowerToughness {
                        power: 2,
                        toughness: 2,
                        restrictions: enum_set!(Restriction::SingleTarget),
                    })
                ]),],
                source: equipment,
                controller: player.clone(),
            },
            Some(ActiveTarget::Battlefield { id: creature })
        )]
    );

    battlefield.apply_action_results(&mut all_cards, &mut modifiers, &mut stack, results);

    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(
        results,
        [StackResult::ModifyCreatures {
            source: equipment,
            targets: vec![creature],
            modifier: ModifierInPlay {
                modifier: BattlefieldModifier {
                    modifier: ModifyBattlefield::AddPowerToughness(AddPowerToughness {
                        power: 2,
                        toughness: 2,
                        restrictions: enum_set!(Restriction::SingleTarget),
                    }),
                    controller: Controller::You,
                    duration: EffectDuration::UntilUnattached,
                },
                controller: player.clone(),
                modifying: vec![]
            },
        }]
    );

    stack.apply_results(&mut all_cards, &mut modifiers, &mut battlefield, results);

    let card = &all_cards[creature];
    assert_eq!(card.card.power(), Some(6));
    assert_eq!(card.card.toughness(), Some(4));

    let creature2 = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
    let _ = battlefield.add(&mut all_cards, &mut modifiers, creature2);

    let card2 = &all_cards[creature2];
    assert_eq!(card2.card.power(), Some(4));
    assert_eq!(card2.card.toughness(), Some(2));

    battlefield.permanent_to_graveyard(&mut all_cards, &mut modifiers, &mut stack, equipment);

    let card = &all_cards[creature];
    assert_eq!(card.card.power(), Some(4));
    assert_eq!(card.card.toughness(), Some(2));

    assert!(battlefield.no_modifiers());

    Ok(())
}
