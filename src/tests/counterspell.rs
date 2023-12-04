use bevy_ecs::system::RunSystemOnce;

use crate::{
    deck::{Deck, DeckDefinition},
    init_world, load_cards,
    player::PlayerId,
    stack::{self, AddToStackEvent, Stack},
};

#[test]
fn resolves_counterspells() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut world = init_world();

    let mut deck = DeckDefinition::default();
    deck.add_card("Counterspell".to_owned(), 2);
    let mut deck = Deck::new(&mut world, PlayerId::default(), &cards, &deck);

    let counterspell_1 = deck.draw().unwrap();
    let counterspell_2 = deck.draw().unwrap();

    world.send_event(AddToStackEvent {
        entry: stack::StackEntry::Spell(counterspell_1),
        target: None,
    });

    world.run_system_once(stack::add_to_stack)?;

    let target = dbg!(world.resource::<Stack>())
        .target_nth(0)
        .expect("Should have a spell on the stack");

    world.send_event(AddToStackEvent {
        entry: stack::StackEntry::Spell(counterspell_2),
        target: Some(stack::Target::Stack(target)),
    });

    world.run_system_once(stack::add_to_stack)?;
    world.run_system_once(stack::resolve_1)?;

    assert!(world.resource::<Stack>().is_empty());

    world.run_system_once(stack::handle_results)?;

    Ok(())
}
