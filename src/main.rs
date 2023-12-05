#![allow(clippy::single_match)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use bevy_ecs::{
    entity::Entity,
    event::{Event, Events},
    world::World,
};
use include_dir::{include_dir, Dir};

use crate::{
    battlefield::{ActivateAbilityEvent, Battlefield, EtbEvent, PermanentToGraveyardEvent},
    card::Card,
    stack::{AddToStackEvent, Stack, StackResult},
};

#[cfg(test)]
pub mod tests;

pub mod abilities;
pub mod activated_ability;
pub mod battlefield;
pub mod card;
pub mod controller;
pub mod cost;
pub mod deck;
pub mod effects;
pub mod mana;
pub mod player;
pub mod protogen;
pub mod stack;
pub mod targets;
pub mod types;

#[derive(Debug, Event)]
pub enum FollowupWork {
    ChooseTargetThenEtb {
        valid_targets: Vec<Entity>,
        targets_for: Entity,
        up_to: usize,
    },

    Etb {
        events: Vec<EtbEvent>,
    },

    Graveyard {
        events: Vec<PermanentToGraveyardEvent>,
    },
}

static CARD_DEFINITIONS: Dir = include_dir!("cards");

pub type Cards = HashMap<String, Card>;

pub fn load_cards() -> anyhow::Result<Cards> {
    let mut cards = Cards::default();
    for card in CARD_DEFINITIONS.entries().iter() {
        let card_file = card
            .as_file()
            .ok_or_else(|| anyhow!("Non-file entry in cards directory"))?;

        let card: protogen::card::Card = protobuf::text_format::parse_from_str(
            card_file
                .contents_utf8()
                .ok_or_else(|| anyhow!("Non utf-8 text proto"))?,
        )
        .with_context(|| format!("Parsing file: {}", card_file.path().display()))?;

        cards.insert(
            card.name.to_owned(),
            card.try_into()
                .with_context(|| format!("Validating file: {}", card_file.path().display()))?,
        );
    }

    Ok(cards)
}

pub fn init_world() -> World {
    let stack = Stack::default();
    let battlefield = Battlefield::default();

    let mut world = World::default();
    world.insert_resource(battlefield);
    world.insert_resource(stack);

    // Keep sorted
    world.init_resource::<Events<ActivateAbilityEvent>>();
    world.init_resource::<Events<AddToStackEvent>>();
    world.init_resource::<Events<EtbEvent>>();
    world.init_resource::<Events<FollowupWork>>();
    world.init_resource::<Events<PermanentToGraveyardEvent>>();
    world.init_resource::<Events<StackResult>>();

    world
}

fn main() -> anyhow::Result<()> {
    let cards = load_cards()?;
    dbg!(&cards);
    dbg!(cards.get("Elesh Norn, Grand Cenobite"));

    Ok(())
}
