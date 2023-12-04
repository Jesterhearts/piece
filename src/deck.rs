use std::collections::{HashMap, VecDeque};

use bevy_ecs::{entity::Entity, world::World};
use rand::{seq::SliceRandom, thread_rng};

use crate::{
    card::Card,
    player::{Controller, Owner, PlayerId},
};

#[derive(Debug, Default)]
pub struct DeckDefinition {
    pub cards: HashMap<String, usize>,
}

impl DeckDefinition {
    pub fn add_card(&mut self, name: String, count: usize) {
        self.cards.insert(name, count);
    }
}

#[derive(Debug)]
pub struct Deck {
    pub cards: VecDeque<Entity>,
}

impl Deck {
    pub fn new(
        world: &mut World,
        player: PlayerId,
        card_definitions: &HashMap<String, Card>,
        definition: &DeckDefinition,
    ) -> Self {
        let mut cards = VecDeque::default();
        for (name, count) in definition.cards.iter() {
            for _ in 0..*count {
                let card = card_definitions.get(name).expect("Valid card name");
                let mut entity = world.spawn(card.clone());
                entity.insert(Owner(player)).insert(Controller(player));
                cards.push_back(entity.id());
            }
        }

        Self { cards }
    }

    pub fn shuffle(&mut self) {
        self.cards.make_contiguous().shuffle(&mut thread_rng())
    }

    pub fn draw(&mut self) -> Option<Entity> {
        self.cards.pop_back()
    }
}
