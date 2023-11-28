use std::rc::Rc;

use rand::{seq::SliceRandom, thread_rng};

use crate::card::Card;

#[derive(Debug)]
pub struct Deck {
    pub cards: Vec<Rc<Card>>,
}

impl Deck {
    pub fn new(cards: Vec<Rc<Card>>) -> Self {
        Self { cards }
    }

    pub fn shuffle(&mut self) {
        self.cards.shuffle(&mut thread_rng())
    }

    pub fn draw(&mut self) -> Option<Rc<Card>> {
        self.cards.pop()
    }
}
