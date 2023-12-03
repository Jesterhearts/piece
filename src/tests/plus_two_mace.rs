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

    let creature2 = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
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
            EffectsInPlay {
                effects: vec![ActivatedAbilityEffect::Equip(vec![
                    ModifyBattlefield::AddPowerToughness(AddPowerToughness {
                        power: 2,
                        toughness: 2
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
                        toughness: 2
                    }),
                    controller: Controller::You,
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                    restrictions: Default::default()
                },
                controller: player.clone(),
                modifying: vec![]
            },
        }]
    );

    let Some(StackResult::ModifyCreatures {
        source,
        targets,
        modifier,
    }) = results.into_iter().next()
    else {
        unreachable!()
    };

    let modifier = modifiers.add_modifier(modifier);
    battlefield.apply_modifier_to_targets(
        &mut all_cards,
        &mut modifiers,
        source,
        modifier,
        targets,
    );

    let card = &all_cards[creature];
    assert_eq!(card.card.power(), 6);
    assert_eq!(card.card.toughness(), 4);

    let card2 = &all_cards[creature2];
    assert_eq!(card2.card.power(), 4);
    assert_eq!(card2.card.toughness(), 2);

    battlefield.permanent_to_graveyard(&mut all_cards, &mut modifiers, &mut stack, equipment);

    let card = &all_cards[creature];
    assert_eq!(card.card.power(), 4);
    assert_eq!(card.card.toughness(), 2);

    Ok(())
}
