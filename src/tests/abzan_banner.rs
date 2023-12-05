use bevy_ecs::system::RunSystemOnce;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{self, ActivateAbilityEvent},
    deck::{Deck, DeckDefinition},
    init_world, load_cards,
    player::{ManaPool, Owner},
    stack::{self, AddToStackEvent, StackEntry},
};

#[test]
fn sacrifice_draw_card() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Abzan Banner", 2);
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);

    let mut mana_pool = world.query::<&mut ManaPool>().single_mut(&mut world);
    mana_pool.infinite();

    let banner = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(banner),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    world.send_event(ActivateAbilityEvent {
        card: banner,
        index: 1,
        targets: vec![],
        choice: None,
    });

    world.run_system_once(battlefield::activate_ability);
    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);

    assert!(world.query::<&Deck>().single(&world).cards.is_empty());

    Ok(())
}

#[test]
fn add_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Abzan Banner", 1);
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);

    let banner = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    world.send_event(AddToStackEvent {
        entry: StackEntry::Spell(banner),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);
    world.run_system_once(battlefield::handle_events);

    world.send_event(ActivateAbilityEvent {
        card: banner,
        index: 0,
        targets: vec![],
        choice: Some(0),
    });

    world.run_system_once(battlefield::activate_ability);
    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);

    let mana_pool = world.query::<&ManaPool>().single(&world);
    assert_eq!(mana_pool.white_mana, 1);
    assert_eq!(mana_pool.black_mana, 0);
    assert_eq!(mana_pool.green_mana, 0);

    Ok(())
}
