use std::collections::{HashMap, VecDeque};

use rand::{seq::SliceRandom, thread_rng};

use crate::{
    in_play::{Database, ExileReason},
    protogen::ids::Owner,
    protogen::{effects::Duration, ids::CardId},
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

    pub fn build_deck(&self, db: &mut Database, cards: &Cards, player: Owner) -> Library {
        let mut deck = VecDeque::default();
        for (card, count) in self.cards.iter() {
            for _ in 0..*count {
                let id = CardId::upload(db, cards, player.clone(), card);
                deck.push_back(id);
            }
        }

        Library::new(deck)
    }
}

#[derive(Debug, Default)]
pub struct Library {
    pub(crate) cards: VecDeque<CardId>,
}

impl Library {
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

    pub(crate) fn place_on_top(db: &mut Database, player: &Owner, card: CardId) {
        if card.move_to_library(db) {
            db.all_players[player].library.cards.push_back(card);
        }
    }

    pub(crate) fn place_under_top(db: &mut Database, player: &Owner, card: CardId, n: usize) {
        if card.move_to_library(db) {
            let library = &mut db.all_players[player].library;
            library.cards.insert(library.cards.len() - n, card);
        }
    }

    pub(crate) fn place_on_bottom(db: &mut Database, player: &Owner, card: CardId) {
        if card.move_to_library(db) {
            db.all_players[player].library.cards.push_front(card);
        }
    }

    pub(crate) fn exile_top_card(
        db: &mut Database,
        player: &Owner,
        source: &CardId,
        reason: Option<ExileReason>,
    ) -> Option<CardId> {
        if let Some(card) = db.all_players[player].library.cards.pop_back() {
            card.move_to_exile(db, source, reason, Duration::PERMANENTLY);
            Some(card)
        } else {
            None
        }
    }

    pub(crate) fn reveal_top(db: &mut Database, player: &Owner) -> Option<CardId> {
        if let Some(card) = db.all_players[player].library.cards.back().cloned() {
            {
                db[&card].revealed = true;
            };
            Some(card)
        } else {
            None
        }
    }

    pub(crate) fn draw(&mut self) -> Option<CardId> {
        self.cards.pop_back()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.cards.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub(crate) fn remove(&mut self, card: &CardId) {
        self.cards.retain(|deck| deck != card);
    }
}
