#![allow(clippy::single_match)]

use std::{collections::HashMap, rc::Rc};

use anyhow::{anyhow, Context};
use include_dir::{include_dir, Dir};

use crate::card::Card;

pub mod activated_ability;
pub mod battlefield;
pub mod card;
pub mod deck;
pub mod hand;
pub mod mana;
pub mod player;
pub mod stack;

static CARD_DEFINITIONS: Dir = include_dir!("cards");

fn load_cards() -> anyhow::Result<HashMap<String, Rc<Card>>> {
    let mut cards: HashMap<String, Rc<Card>> = Default::default();
    for card in CARD_DEFINITIONS.entries().iter() {
        let card = card
            .as_file()
            .ok_or_else(|| anyhow!("Non-file entry in cards directory"))?;
        let card: Rc<Card> = Rc::new(
            serde_yaml::from_slice(card.contents())
                .with_context(|| format!("Unpacking file: {}", card.path().display()))?,
        );
        cards.insert(card.name.clone(), card);
    }
    Ok(cards)
}

fn main() -> anyhow::Result<()> {
    let cards = load_cards()?;
    dbg!(&cards);

    Ok(())
}
