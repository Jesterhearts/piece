use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use crate::{
    card::{Card, Effect},
    deck::{Deck, DeckDefinition},
    player::PlayerRef,
    Cards,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CardInPlay {
    pub card: Rc<Card>,
    pub controller: PlayerRef,
    pub owner: PlayerRef,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EffectInPlay {
    pub effect: Effect,
    pub controller: PlayerRef,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct CardId(usize);

#[derive(Default)]
pub struct AllCards {
    pub cards: HashMap<CardId, CardInPlay>,
    next_id: usize,
}

impl std::ops::Index<CardId> for AllCards {
    type Output = CardInPlay;

    fn index(&self, index: CardId) -> &Self::Output {
        self.cards.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<CardId> for AllCards {
    fn index_mut(&mut self, index: CardId) -> &mut Self::Output {
        self.cards.get_mut(&index).unwrap()
    }
}

impl AllCards {
    pub fn add_deck(&mut self, cards: &Cards, definition: &DeckDefinition) -> Deck {
        let mut deck = VecDeque::default();
        for (card, count) in definition.cards.iter() {
            for _ in 0..*count {
                let id = self.add(cards, definition.owner.clone(), card);
                deck.push_back(id);
            }
        }

        Deck::new(deck)
    }

    #[must_use]
    pub fn add(&mut self, cards: &Cards, owner: PlayerRef, name: &str) -> CardId {
        let id = self.next_id();
        self.cards.insert(
            id,
            CardInPlay {
                card: cards.get(name).expect("Valid card name").clone(),
                controller: owner.clone(),
                owner,
            },
        );
        id
    }

    fn next_id(&mut self) -> CardId {
        let id = self.next_id;
        self.next_id += 1;
        CardId(id)
    }
}
