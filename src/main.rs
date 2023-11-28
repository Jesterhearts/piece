#![allow(clippy::single_match)]

use std::{cell::RefCell, collections::HashMap, rc::Rc};

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

fn main() -> anyhow::Result<()> {
    let mut cards: HashMap<String, Rc<Card>> = Default::default();
    for card in std::fs::read_dir("cards/").unwrap() {
        let card = dbg!(std::fs::File::open(card?.path())?);
        let card: Rc<Card> = Rc::new(serde_yaml::from_reader(card)?);
        cards.insert(card.name.clone(), card);
    }
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
    let player = Rc::new(RefCell::new(Player::new(Deck::new(deck), 0)));
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
