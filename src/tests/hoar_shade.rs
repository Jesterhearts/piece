use enumset::enum_set;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    controller::Controller,
    deck::Deck,
    effects::{
        ActivatedAbilityEffect, AddPowerToughness, BattlefieldModifier, EffectDuration,
        ModifyBattlefield,
    },
    in_play::{AllCards, AllModifiers, EffectsInPlay, ModifierInPlay, ModifierType},
    load_cards,
    player::Player,
    stack::{Stack, StackResult},
    targets::Restriction,
};

#[test]
fn add_p_t_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());
    player.borrow_mut().infinite_mana();

    let shade1 = all_cards.add(&cards, player.clone(), "Hoar Shade");
    let shade2 = all_cards.add(&cards, player.clone(), "Hoar Shade");

    let results = battlefield.add(&mut all_cards, &mut modifiers, shade1, vec![]);
    assert_eq!(results, []);

    let results = battlefield.add(&mut all_cards, &mut modifiers, shade2, vec![]);
    assert_eq!(results, []);

    let card = battlefield.select_card(0);
    let results = battlefield.activate_ability(card, &all_cards, &stack, 0);

    assert_eq!(
        results,
        [UnresolvedActionResult::AddToStack {
            card,
            effects: EffectsInPlay {
                effects: vec![ActivatedAbilityEffect::BattlefieldModifier(
                    BattlefieldModifier {
                        modifier: ModifyBattlefield::AddPowerToughness(AddPowerToughness {
                            power: 1,
                            toughness: 1,
                        }),
                        controller: Controller::You,
                        duration: EffectDuration::UntilEndOfTurn,
                        restrictions: enum_set!(Restriction::Self_),
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
                    modifier: ModifyBattlefield::AddPowerToughness(AddPowerToughness {
                        power: 1,
                        toughness: 1,
                    }),
                    controller: Controller::You,
                    duration: EffectDuration::UntilEndOfTurn,
                    restrictions: enum_set!(Restriction::Self_),
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
    assert_eq!(card.card.power(), Some(2));
    assert_eq!(card.card.toughness(), Some(3));

    let card = battlefield.select_card(1);
    let card = &all_cards[card];
    assert_eq!(card.card.power(), Some(1));
    assert_eq!(card.card.toughness(), Some(2));

    battlefield.end_turn(&mut all_cards, &mut modifiers);

    let card = battlefield.select_card(0);
    let card = &all_cards[card];
    assert_eq!(card.card.power(), Some(1));
    assert_eq!(card.card.toughness(), Some(2));

    Ok(())
}
