#![allow(clippy::single_match)]

use std::{collections::HashMap, rc::Rc};

use anyhow::{anyhow, Context};
use include_dir::{include_dir, Dir};

use crate::{
    battlefield::Battlefield,
    card::{Card, PlayedCard},
    deck::Deck,
    player::Player,
    stack::Stack,
};

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

    let mut stack = Stack::default();

    let mut battlefield = Battlefield::default();

    let deck = vec![
        cards.get("Allosaurus Shepherd").cloned().unwrap(),
        cards.get("Counterspell").cloned().unwrap(),
        cards.get("Forest").cloned().unwrap(),
        cards.get("Forest").cloned().unwrap(),
        cards.get("Forest").cloned().unwrap(),
        cards.get("Forest").cloned().unwrap(),
        cards.get("Forest").cloned().unwrap(),
    ];
    let player = Player::new_ref(Deck::new(deck), 0);
    player.borrow_mut().draw_initial_hand();

    let played = dbg!(player.borrow_mut().play_card(0, &stack, &battlefield, None));
    if let Some(played) = played {
        if played.uses_stack() {
            stack.push_card(
                PlayedCard {
                    card: played,
                    controller: player.clone(),
                    owner: player.clone(),
                },
                None,
            );
        } else {
            battlefield.add(PlayedCard {
                card: played,
                controller: player.clone(),
                owner: player.clone(),
            });
        }
    }

    dbg!(&battlefield);

    dbg!(&stack);
    stack.resolve_1(&mut battlefield);
    dbg!(stack);

    Ok(())
}
