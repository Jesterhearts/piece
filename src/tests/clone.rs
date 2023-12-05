use std::collections::HashSet;

use bevy_ecs::{event::Events, query::With, system::RunSystemOnce};
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{self, BattlefieldId, EtbEvent},
    card::CardName,
    deck::{Deck, DeckDefinition},
    init_world, load_cards,
    player::Owner,
    stack::{self, AddToStackEvent, StackEntry},
    FollowupWork,
};

#[test]
fn etb_clones() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Clone", 1);
    deck.add_card("Alpine Grizzly", 1);
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);

    let bear = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();
    let clone = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(bear),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(clone),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    for followup in world
        .resource_mut::<Events<FollowupWork>>()
        .drain()
        .collect::<Vec<_>>()
    {
        match followup {
            FollowupWork::ChooseTargetThenEtb {
                valid_targets,
                targets_for,
                up_to,
            } => {
                assert_eq!(up_to, 1);
                assert_eq!(valid_targets.len(), 1);
                world.send_event(EtbEvent {
                    card: targets_for,
                    targets: Some(valid_targets),
                })
            }
            FollowupWork::Etb { events } => {
                assert!(events.is_empty());
            }
            FollowupWork::Graveyard { battlefield, stack } => {
                assert!(battlefield.is_empty());
                assert!(stack.is_empty());
            }
        }
    }

    world.run_system_once(battlefield::handle_events);

    world.run_system_once(battlefield::handle_sba);
    world.run_system_once(battlefield::handle_events);

    let mut on_battlefield = world.query_filtered::<&CardName, With<BattlefieldId>>();
    let on_battlefield = on_battlefield
        .iter(&world)
        .map(|card| (**card).clone())
        .collect::<HashSet<_>>();

    assert_eq!(on_battlefield.len(), 2);
    assert!(on_battlefield.contains("Alpine Grizzly"));
    assert!(on_battlefield.contains("Clone"));

    Ok(())
}

#[test]
fn etb_no_targets_dies() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Clone", 1);
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);

    let clone = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(clone),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    for followup in world
        .resource_mut::<Events<FollowupWork>>()
        .drain()
        .collect::<Vec<_>>()
    {
        match followup {
            FollowupWork::ChooseTargetThenEtb {
                valid_targets,
                targets_for,
                up_to,
            } => {
                assert_eq!(up_to, 1);
                assert_eq!(valid_targets.len(), 0);
                world.send_event(EtbEvent {
                    card: targets_for,
                    targets: Some(vec![]),
                })
            }
            FollowupWork::Etb { events } => {
                assert!(events.is_empty());
            }
            FollowupWork::Graveyard { battlefield, stack } => {
                assert!(battlefield.is_empty());
                assert!(stack.is_empty());
            }
        }
    }

    world.run_system_once(battlefield::handle_events);

    let mut on_battlefield = world.query_filtered::<&CardName, With<BattlefieldId>>();
    let on_battlefield = on_battlefield
        .iter(&world)
        .map(|card| (**card).clone())
        .collect::<HashSet<_>>();

    assert_eq!(on_battlefield.len(), 1);
    assert!(on_battlefield.contains("Clone"));

    world.run_system_once(battlefield::handle_sba);
    world.run_system_once(battlefield::handle_events);

    let mut on_battlefield = world.query_filtered::<&CardName, With<BattlefieldId>>();
    let on_battlefield = on_battlefield
        .iter(&world)
        .map(|card| (**card).clone())
        .collect::<HashSet<_>>();

    assert!(on_battlefield.is_empty());

    Ok(())
}
