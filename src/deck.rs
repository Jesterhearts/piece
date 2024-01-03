use std::collections::{HashMap, VecDeque};

use rand::{seq::SliceRandom, thread_rng};

use crate::{
    effects::EffectDuration,
    in_play::{CardId, Database, ExileReason},
    player::Owner,
    Cards,
};

#[derive(Debug, Default)]
pub struct DeckDefinition {
    pub(crate) cards: HashMap<String, usize>,
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
    pub(crate) cards: VecDeque<CardId>,
}

impl Deck {
    pub(crate) fn empty() -> Self {
        Self {
            cards: Default::default(),
        }
    }

    pub(crate) fn new(cards: VecDeque<CardId>) -> Self {
        Self { cards }
    }

    pub(crate) fn shuffle(&mut self) {
        self.cards.make_contiguous().shuffle(&mut thread_rng())
    }

    pub(crate) fn place_on_top(&mut self, db: &mut Database, card: CardId) {
        if card.move_to_library(db) {
            self.cards.push_back(card);
        }
    }

    pub(crate) fn place_under_top(&mut self, db: &mut Database, card: CardId, n: usize) {
        if card.move_to_library(db) {
            self.cards.insert(self.cards.len() - n, card);
        }
    }

    pub(crate) fn place_on_bottom(&mut self, db: &mut Database, card: CardId) {
        if card.move_to_library(db) {
            self.cards.push_front(card);
        }
    }

    pub(crate) fn draw(&mut self) -> Option<CardId> {
        self.cards.pop_back()
    }

    #[allow(unused)]
    pub(crate) fn len(&self) -> usize {
        self.cards.len()
    }

    #[allow(unused)]
    pub(crate) fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub(crate) fn remove(&mut self, card: CardId) {
        self.cards.retain(|deck| *deck != card);
    }

    pub(crate) fn reveal_top(&self, db: &mut Database) -> Option<CardId> {
        if let Some(card) = self.cards.back() {
            card.reveal(db);
            Some(*card)
        } else {
            None
        }
    }

    pub(crate) fn exile_top_card(
        &mut self,
        db: &mut Database,
        source: CardId,
        reason: Option<ExileReason>,
    ) -> Option<CardId> {
        if let Some(card) = self.cards.pop_back() {
            card.move_to_exile(db, source, reason, EffectDuration::Permanently);
            Some(card)
        } else {
            None
        }
    }
}
