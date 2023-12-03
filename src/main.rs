#![allow(clippy::single_match)]

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use include_dir::{include_dir, Dir};

use crate::card::Card;

pub mod abilities;
pub mod battlefield;
pub mod card;
pub mod controller;
pub mod cost;
pub mod deck;
pub mod effects;
pub mod hand;
pub mod in_play;
pub mod mana;
pub mod player;
pub mod protogen;
pub mod stack;
pub mod targets;
pub mod types;

#[cfg(test)]
pub mod tests;

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

fn main() -> anyhow::Result<()> {
    let cards = load_cards()?;
    dbg!(&cards);
    dbg!(cards.get("Elesh Norn, Grand Cenobite"));

    Ok(())
}
