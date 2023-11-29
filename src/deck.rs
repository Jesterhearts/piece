use std::collections::{HashMap, VecDeque};

use rand::{seq::SliceRandom, thread_rng};

use crate::{in_play::CardId, player::PlayerRef};

pub struct DeckDefinition {
    pub cards: HashMap<String, usize>,
    pub owner: PlayerRef,
}

#[derive(Debug)]
pub struct Deck {
    pub cards: VecDeque<CardId>,
}

impl Deck {
    pub fn empty() -> Self {
        Self {
            cards: Default::default(),
        }
    }

    pub fn new(cards: VecDeque<CardId>) -> Self {
        Self { cards }
    }

    pub fn shuffle(&mut self) {
        self.cards.make_contiguous().shuffle(&mut thread_rng())
    }

    pub fn draw(&mut self) -> Option<CardId> {
        self.cards.pop_back()
    }
}
