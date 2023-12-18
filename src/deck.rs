use std::collections::{HashMap, VecDeque};

use rand::{seq::SliceRandom, thread_rng};

use crate::{
    in_play::{CardId, Database},
    player::Owner,
    Cards,
};

#[derive(Debug, Default)]
pub struct DeckDefinition {
    pub cards: HashMap<String, usize>,
}

impl DeckDefinition {
    pub fn add_card(&mut self, name: String, count: usize) {
        self.cards.insert(name, count);
    }

    pub fn build_deck(&self, db: &mut Database, cards: &Cards, player: Owner) -> Deck {
        let mut deck = VecDeque::default();
        for (card, count) in self.cards.iter() {
            for _ in 0..*count {
                let id = CardId::upload(db, cards, player, card);
                deck.push_back(id);
            }
        }

        Deck::new(deck)
    }
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

    pub fn place_on_top(&mut self, db: &mut Database, card: CardId) {
        if card.move_to_library(db) {
            self.cards.push_back(card);
        }
    }

    pub fn draw(&mut self) -> Option<CardId> {
        self.cards.pop_back()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn remove(&mut self, card: CardId) {
        self.cards.retain(|deck| *deck != card);
    }

    pub fn reveal_top(&self, db: &mut Database) -> Option<CardId> {
        if let Some(card) = self.cards.back() {
            card.reveal(db);
            Some(*card)
        } else {
            None
        }
    }
}
