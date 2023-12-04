use std::collections::HashSet;

use bevy_ecs::{query::With, system::RunSystemOnce};
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{self, Battlefield, BattlefieldId},
    card::Card,
    deck::{Deck, DeckDefinition},
    init_world, load_cards,
    player::PlayerId,
    stack::{self, AddToStackEvent, Stack, StackEntry, Target},
};

#[test]
fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Allosaurus Shepherd", 1);
    deck.add_card("Counterspell", 1);
    let deck = Deck::add_to_world(&mut world, PlayerId::new(), &cards, &deck);

    let counterspell = world.get_mut::<Deck>(deck).unwrap().draw().unwrap();
    let creature = world.get_mut::<Deck>(deck).unwrap().draw().unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(creature),
        target: None,
    });
    world.run_system_once(stack::add_to_stack)?;

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(counterspell),
        target: world.resource::<Stack>().target_nth(0).map(Target::Stack),
    });

    world.run_system_once(stack::add_to_stack)?;
    assert_eq!(world.resource::<Stack>().len(), 2);

    world.run_system_once(stack::resolve_1)?;
    world.run_system_once(stack::handle_results)?;
    assert_eq!(world.resource::<Stack>().len(), 1);

    world.run_system_once(stack::resolve_1)?;
    world.run_system_once(stack::handle_results)?;
    assert!(world.resource::<Stack>().is_empty());

    world.resource::<Battlefield>();

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_green_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Allosaurus Shepherd", 1);
    deck.add_card("Alpine Grizzly", 1);
    deck.add_card("Counterspell", 1);
    let deck = Deck::add_to_world(&mut world, PlayerId::new(), &cards, &deck);

    let counterspell = world.get_mut::<Deck>(deck).unwrap().draw().unwrap();
    let bear = world.get_mut::<Deck>(deck).unwrap().draw().unwrap();
    let creature = world.get_mut::<Deck>(deck).unwrap().draw().unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(creature),
        target: None,
    });

    world.run_system_once(stack::add_to_stack)?;
    world.run_system_once(stack::resolve_1)?;
    world.run_system_once(stack::handle_results)?;
    world.run_system_once(battlefield::handle_events)?;

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(bear),
        target: None,
    });

    world.run_system_once(stack::add_to_stack)?;

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(counterspell),
        target: world.resource::<Stack>().target_nth(0).map(Target::Stack),
    });

    world.run_system_once(stack::add_to_stack)?;

    world.run_system_once(stack::resolve_1)?;
    world.run_system_once(stack::handle_results)?;
    world.run_system_once(battlefield::handle_events)?;

    world.run_system_once(stack::resolve_1)?;
    world.run_system_once(stack::handle_results)?;
    world.run_system_once(battlefield::handle_events)?;

    let mut on_battlefield = world.query_filtered::<&Card, With<BattlefieldId>>();
    let on_battlefield = on_battlefield
        .iter(&world)
        .map(|card| card.name.clone())
        .collect::<HashSet<_>>();

    assert_eq!(on_battlefield.len(), 2);
    assert!(on_battlefield.contains("Allosaurus Shepherd"));
    assert!(on_battlefield.contains("Alpine Grizzly"));

    Ok(())
}

#[test]
fn resolves_counterspells_respecting_green_uncounterable_other_player() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Allosaurus Shepherd", 1);
    deck.add_card("Counterspell", 1);
    let deck = Deck::add_to_world(&mut world, PlayerId::new(), &cards, &deck);

    let counterspell = world.get_mut::<Deck>(deck).unwrap().draw().unwrap();
    let creature = world.get_mut::<Deck>(deck).unwrap().draw().unwrap();

    let mut deck = DeckDefinition::default();
    deck.add_card("Alpine Grizzly", 1);
    let deck = Deck::add_to_world(&mut world, PlayerId::new(), &cards, &deck);
    let bear = world.get_mut::<Deck>(deck).unwrap().draw().unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(creature),
        target: None,
    });

    world.run_system_once(stack::add_to_stack)?;
    world.run_system_once(stack::resolve_1)?;
    world.run_system_once(stack::handle_results)?;
    world.run_system_once(battlefield::handle_events)?;

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(bear),
        target: None,
    });

    world.run_system_once(stack::add_to_stack)?;

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(counterspell),
        target: world.resource::<Stack>().target_nth(0).map(Target::Stack),
    });

    world.run_system_once(stack::add_to_stack)?;

    world.run_system_once(stack::resolve_1)?;
    world.run_system_once(stack::handle_results)?;
    world.run_system_once(battlefield::handle_events)?;

    assert!(world.resource::<Stack>().is_empty());

    let mut on_battlefield = world.query_filtered::<&Card, With<BattlefieldId>>();
    let on_battlefield = on_battlefield
        .iter(&world)
        .map(|card| card.name.clone())
        .collect::<HashSet<_>>();

    assert_eq!(on_battlefield.len(), 1);
    assert!(on_battlefield.contains("Allosaurus Shepherd"));

    Ok(())
}

/*
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
    let _ = battlefield.add(&mut all_cards, &mut modifiers, card);

    let card = battlefield.select_card(0);
    let results = battlefield.activate_ability(card, &all_cards, &stack, 0, None);

    assert_eq!(
        results,
        [ActionResult::AddToStack(
            card,
            EffectsInPlay {
                effects: vec![
                    ActivatedAbilityEffect::BattlefieldModifier(BattlefieldModifier {
                        modifier: ModifyBattlefield::ModifyBasePowerToughness(
                            ModifyBasePowerToughness {
                                targets: vec![Subtype::Elf],
                                power: 5,
                                toughness: 5,
                                restrictions: Default::default(),
                            }
                        ),
                        controller: Controller::You,
                        duration: EffectDuration::UntilEndOfTurn,
                    }),
                    ActivatedAbilityEffect::BattlefieldModifier(BattlefieldModifier {
                        modifier: ModifyBattlefield::AddCreatureSubtypes(AddCreatureSubtypes {
                            targets: vec![Subtype::Elf],
                            types: vec![Subtype::Dinosaur],
                            restrictions: Default::default(),
                        }),
                        controller: Controller::You,
                        duration: EffectDuration::UntilEndOfTurn,
                    })
                ],
                source: card,
                controller: player.clone(),
            },
            None
        )]
    );

    battlefield.apply_action_results(&mut all_cards, &mut modifiers, &mut stack, results);

    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(
        results,
        [
            StackResult::ApplyToBattlefield {
                source: card,
                modifier: ModifierInPlay {
                    modifier: BattlefieldModifier {
                        modifier: ModifyBattlefield::ModifyBasePowerToughness(
                            ModifyBasePowerToughness {
                                targets: vec![Subtype::Elf],
                                power: 5,
                                toughness: 5,
                                restrictions: Default::default(),
                            }
                        ),
                        controller: Controller::You,
                        duration: EffectDuration::UntilEndOfTurn,
                    },
                    controller: player.clone(),
                    modifying: Default::default(),
                },
            },
            StackResult::ApplyToBattlefield {
                source: card,
                modifier: ModifierInPlay {
                    modifier: BattlefieldModifier {
                        modifier: ModifyBattlefield::AddCreatureSubtypes(AddCreatureSubtypes {
                            targets: vec![Subtype::Elf],
                            types: vec![Subtype::Dinosaur],
                            restrictions: Default::default(),
                        }),
                        controller: Controller::You,
                        duration: EffectDuration::UntilEndOfTurn,
                    },
                    controller: player.clone(),
                    modifying: Default::default(),
                }
            }
        ]
    );

    stack.apply_results(&mut all_cards, &mut modifiers, &mut battlefield, results);

    let card = battlefield.select_card(0);
    let card = &all_cards[card];
    assert_eq!(card.card.power(), 5);
    assert_eq!(card.card.toughness(), 5);

    assert_eq!(
        card.card.subtypes(),
        HashSet::from([Subtype::Elf, Subtype::Shaman, Subtype::Dinosaur])
    );

    battlefield.end_turn(&mut all_cards, &mut modifiers);

    let card = battlefield.select_card(0);
    let card = &all_cards[card];
    assert_eq!(card.card.power(), 1);
    assert_eq!(card.card.toughness(), 1);

    assert_eq!(
        card.card.subtypes,
        HashSet::from([Subtype::Elf, Subtype::Shaman])
    );

    Ok(())
}
*/
