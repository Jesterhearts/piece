use std::collections::HashSet;

use bevy_ecs::{
    query::With,
    system::{Query, RunSystemOnce},
};
use enumset::{enum_set, EnumSet};
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{self, ActivateAbilityEvent, Battlefield, BattlefieldId},
    card::{
        CardName, CardSubtypes, ModifyingPower, ModifyingSubtypeSet, ModifyingSubtypes,
        ModifyingToughness, Power, PowerModifier, Toughness, ToughnessModifier,
    },
    deck::{Deck, DeckDefinition},
    init_world, load_cards,
    player::{ManaPool, Owner},
    stack::{self, AddToStackEvent, Stack, StackEntry, Targets},
    types::Subtype,
};

#[test]
fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Allosaurus Shepherd", 1);
    deck.add_card("Counterspell", 1);
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);

    let counterspell = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();
    let creature = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(creature),
        target: None,
        choice: None,
    });
    world.run_system_once(stack::add_to_stack);

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(counterspell),
        target: world
            .resource::<Stack>()
            .target_nth(0)
            .map(|target| Targets::Stack(vec![target])),
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    assert_eq!(world.resource::<Stack>().len(), 2);

    world.run_system_once(stack::resolve_1);
    assert_eq!(world.resource::<Stack>().len(), 1);

    world.run_system_once(stack::resolve_1);
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
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);

    let counterspell = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();
    let bear = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();
    let creature = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(creature),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(bear),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(counterspell),
        target: world
            .resource::<Stack>()
            .target_nth(0)
            .map(|target| Targets::Stack(vec![target])),
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);

    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    assert!(!world.resource::<Stack>().is_empty());

    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    let mut on_battlefield = world.query_filtered::<&CardName, With<BattlefieldId>>();
    let on_battlefield = on_battlefield
        .iter(&world)
        .map(|card| (**card).clone())
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
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);

    let counterspell = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();
    let creature = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    let mut deck = DeckDefinition::default();
    deck.add_card("Alpine Grizzly", 1);
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);
    let bear = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(creature),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(bear),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(counterspell),
        target: world
            .resource::<Stack>()
            .target_nth(0)
            .map(|target| Targets::Stack(vec![target])),
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    assert!(world.resource::<Stack>().is_empty());

    let mut on_battlefield = world.query_filtered::<&CardName, With<BattlefieldId>>();
    let on_battlefield = on_battlefield
        .iter(&world)
        .map(|card| (**card).clone())
        .collect::<HashSet<_>>();

    assert_eq!(on_battlefield.len(), 1);
    assert!(on_battlefield.contains("Allosaurus Shepherd"));

    Ok(())
}

#[test]
fn modify_base_p_t_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Allosaurus Shepherd", 1);
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);

    let mut mana_pool = world.query::<&mut ManaPool>().single_mut(&mut world);
    mana_pool.infinite();

    let creature = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(creature),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    world.send_event(ActivateAbilityEvent {
        card: creature,
        index: 0,
        targets: vec![],
        choice: None,
    });

    world.run_system_once(battlefield::activate_ability);
    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);

    fn query_pt(
        query: Query<(
            &CardSubtypes,
            &Toughness,
            &Power,
            &ModifyingSubtypes,
            &ModifyingToughness,
            &ModifyingPower,
        )>,
        toughness_modifiers: Query<&ToughnessModifier>,
        power_modifiers: Query<&PowerModifier>,
        subtype_modifiers: Query<&ModifyingSubtypeSet>,
    ) -> (EnumSet<Subtype>, Option<i32>, Option<i32>) {
        let (subtypes, toughness, power, subtypes_mod, toughness_mod, power_mod) = query.single();
        (
            subtypes_mod.union(subtypes, &subtype_modifiers),
            toughness_mod.toughness(toughness, &toughness_modifiers),
            power_mod.power(power, &power_modifiers),
        )
    }

    let result = world.run_system_once(query_pt);
    assert_eq!(
        result,
        (
            enum_set!(Subtype::Elf | Subtype::Shaman | Subtype::Dinosaur),
            Some(5),
            Some(5)
        )
    );

    world.run_system_once(battlefield::end_turn);
    let result = world.run_system_once(query_pt);
    assert_eq!(
        result,
        (enum_set!(Subtype::Elf | Subtype::Shaman), Some(1), Some(1))
    );

    Ok(())
}
