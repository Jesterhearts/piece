use std::{cell::RefCell, rc::Rc};

use derive_more::Deref;

use crate::{
    battlefield::Battlefield,
    card::Card,
    deck::Deck,
    hand::Hand,
    mana::Mana,
    stack::{ActiveTarget, Stack},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ManaPool {
    pub white_mana: usize,
    pub blue_mana: usize,
    pub black_mana: usize,
    pub red_mana: usize,
    pub green_mana: usize,
    pub colorless_mana: usize,
}

impl ManaPool {
    pub fn apply(&mut self, mana: Mana) {
        match mana {
            Mana::White => self.white_mana += 1,
            Mana::Blue => self.blue_mana += 1,
            Mana::Black => self.black_mana += 1,
            Mana::Red => self.red_mana += 1,
            Mana::Green => self.green_mana += 1,
            Mana::Colorless => self.colorless_mana += 1,
            Mana::Generic(count) => self.colorless_mana += count,
        }
    }

    fn spend(&mut self, mana: Mana) -> bool {
        match mana {
            Mana::White => {
                let Some(mana) = self.white_mana.checked_sub(1) else {
                    return false;
                };

                self.white_mana = mana;
            }
            Mana::Blue => {
                let Some(mana) = self.blue_mana.checked_sub(1) else {
                    return false;
                };

                self.blue_mana = mana;
            }
            Mana::Black => {
                let Some(mana) = self.black_mana.checked_sub(1) else {
                    return false;
                };

                self.black_mana = mana;
            }
            Mana::Red => {
                let Some(mana) = self.red_mana.checked_sub(1) else {
                    return false;
                };

                self.red_mana = mana;
            }
            Mana::Green => {
                let Some(mana) = self.green_mana.checked_sub(1) else {
                    return false;
                };

                self.green_mana = mana;
            }
            Mana::Colorless => {
                let Some(mana) = self.colorless_mana.checked_sub(1) else {
                    return false;
                };

                self.colorless_mana = mana;
            }
            Mana::Generic(count) => {
                let Some(mana) = self.colorless_mana.checked_sub(count) else {
                    return false;
                };

                self.colorless_mana = mana;
            }
        }

        true
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deref)]
pub struct PlayerRef(Rc<RefCell<Player>>);

impl std::hash::Hash for PlayerRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

impl PartialEq<Player> for PlayerRef {
    fn eq(&self, other: &Player) -> bool {
        &*self.borrow() == other
    }
}

#[derive(Debug)]
pub struct Player {
    pub lands_per_turn: usize,
    pub hexproof: bool,
    pub lands_played: usize,
    pub mana_pool: ManaPool,
    pub hand: Hand,

    pub id: usize,

    pub deck: Deck,
}

impl std::hash::Hash for Player {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Player {}

impl Player {
    pub fn new_ref(deck: Deck, id: usize) -> PlayerRef {
        PlayerRef(Rc::new(RefCell::new(Self {
            lands_per_turn: 1,
            hexproof: false,
            lands_played: 0,
            mana_pool: Default::default(),
            hand: Default::default(),
            id,
            deck,
        })))
    }

    pub fn draw_initial_hand(&mut self) {
        for _ in 0..7 {
            let card = self
                .deck
                .draw()
                .expect("Decks should have at least 7 cards");
            self.hand.contents.push(card);
        }
    }

    /// Returns true if the card was played.
    pub fn play_card(
        &mut self,
        index: usize,
        stack: &Stack,
        battlefield: &Battlefield,
        target: Option<ActiveTarget>,
    ) -> Option<Rc<Card>> {
        let card = &self.hand.contents[index];
        let mana_pool = self.mana_pool;

        for mana in card.cost.mana.iter().copied() {
            if !self.mana_pool.spend(mana) {
                self.mana_pool = mana_pool;
                return None;
            }
        }

        if let Some(target) = target {
            let targets = card.valid_targets(battlefield, stack, self);
            if !targets.contains(&target) {
                return None;
            }
        }

        if card.requires_target() && target.is_none() {
            return None;
        }

        if card.is_land() && self.lands_played >= self.lands_per_turn {
            return None;
        }

        Some(self.hand.contents.remove(index))
    }
}
