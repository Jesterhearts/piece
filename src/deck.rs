use std::collections::{HashMap, VecDeque};

use rand::{seq::SliceRandom, thread_rng};
use rusqlite::Connection;

use crate::{in_play::CardId, player::PlayerId, Cards};

#[derive(Debug, Default)]
pub struct DeckDefinition {
    pub cards: HashMap<String, usize>,
}

impl DeckDefinition {
    pub fn add_card(&mut self, name: String, count: usize) {
        self.cards.insert(name, count);
    }

    pub fn build_deck(
        &self,
        cards: &Cards,
        db: &Connection,
        player: PlayerId,
    ) -> anyhow::Result<Deck> {
        let mut deck = VecDeque::default();
        for (card, count) in self.cards.iter() {
            for _ in 0..*count {
                let id = CardId::upload(db, cards, player, card)?;
                deck.push_back(id);
            }
        }

        Ok(Deck::new(deck))
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

    pub fn place_on_top(&mut self, card: CardId) {
        self.cards.push_back(card);
    }

    pub fn draw(&mut self) -> Option<CardId> {
        self.cards.pop_back()
    }

    pub(crate) fn len(&self) -> usize {
        self.cards.len()
    }
}
