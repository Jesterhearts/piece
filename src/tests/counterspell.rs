use bevy_ecs::system::RunSystemOnce;

use crate::{
    deck::{Deck, DeckDefinition},
    init_world, load_cards,
    player::Owner,
    stack::{self, AddToStackEvent, Stack},
};

#[test]
fn resolves_counterspells() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Counterspell", 2);
    let player = Owner::new(&mut world);
    Deck::add_to_world(&mut world, player, &cards, &deck);

    let counterspell_1 = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();
    let counterspell_2 = world
        .query::<&mut Deck>()
        .single_mut(&mut world)
        .draw()
        .unwrap();

    world.send_event(AddToStackEvent {
        entry: stack::StackEntry::Spell(counterspell_1),
        target: None,
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);

    let target = world
        .resource::<Stack>()
        .target_nth(0)
        .expect("Should have a spell on the stack");

    world.send_event(AddToStackEvent {
        entry: stack::StackEntry::Spell(counterspell_2),
        target: Some(stack::Targets::Stack(vec![target])),
        choice: None,
    });

    world.run_system_once(stack::add_to_stack);
    world.run_system_once(stack::resolve_1);

    assert!(world.resource::<Stack>().is_empty());

    Ok(())
}
