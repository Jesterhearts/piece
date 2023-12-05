use std::collections::VecDeque;

use bevy_ecs::{component::Component, entity::Entity, world::World};
use indexmap::IndexMap;
use rand::{seq::SliceRandom, thread_rng};

use crate::{
    player::{Controller, Owner},
    Cards,
};

#[derive(Debug, Default)]
pub struct DeckDefinition {
    pub cards: IndexMap<String, usize>,
}

impl DeckDefinition {
    pub fn add_card(&mut self, name: impl AsRef<str>, count: usize) {
        self.cards.insert(name.as_ref().to_owned(), count);
    }
}

#[derive(Debug, Component)]
pub struct Deck {
    pub cards: VecDeque<Entity>,
}

impl Deck {
    pub fn add_to_world(
        world: &mut World,
        player: Owner,
        card_definitions: &Cards,
        definition: &DeckDefinition,
    ) {
        let mut cards = VecDeque::default();
        for (name, count) in definition.cards.iter() {
            for _ in 0..*count {
                let card = card_definitions.get(name).expect("Valid card name");
                let mut entity = world.spawn(card.clone());
                entity.insert(player).insert(Controller::from(player));
                cards.push_back(entity.id());
            }
        }

        world.entity_mut(*player).insert(Self { cards });
    }

    pub fn shuffle(&mut self) {
        self.cards.make_contiguous().shuffle(&mut thread_rng())
    }

    pub fn draw(&mut self) -> Option<Entity> {
        self.cards.pop_back()
    }
}
