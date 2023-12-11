use std::{
    collections::{HashMap, HashSet},
    ops::{Index, IndexMut},
    sync::atomic::{AtomicUsize, Ordering},
};

use bevy_ecs::{component::Component, entity::Entity};
use itertools::Itertools;

use crate::{
    abilities::StaticAbility,
    battlefield::{Battlefield, UnresolvedActionResult},
    deck::Deck,
    in_play::{cards, CardId, Database, InHand},
    mana::Mana,
    stack::ActiveTarget,
};

static NEXT_PLAYER_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct Owner(usize);

impl From<Controller> for Owner {
    fn from(value: Controller) -> Self {
        Self(value.0)
    }
}

impl Owner {
    pub fn get_cards<Zone: Component + Ord>(self, db: &mut Database) -> Vec<CardId> {
        db.query::<(Entity, &Owner, &Zone)>()
            .iter(db)
            .sorted_by_key(|(_, _, zone)| *zone)
            .filter_map(|(card, owner, _)| {
                if self == *owner {
                    Some(card.into())
                } else {
                    None
                }
            })
            .collect_vec()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct Controller(usize);

impl From<Owner> for Controller {
    fn from(value: Owner) -> Self {
        Self(value.0)
    }
}

impl Controller {
    pub fn get_cards<Zone: Component + Ord>(self, db: &mut Database) -> Vec<CardId> {
        db.query::<(Entity, &Controller, &Zone)>()
            .iter(db)
            .sorted_by_key(|(_, _, zone)| *zone)
            .filter_map(|(card, owner, _)| {
                if self == *owner {
                    Some(card.into())
                } else {
                    None
                }
            })
            .collect_vec()
    }
}

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
            Mana::White => self.white_mana = self.white_mana.saturating_add(1),
            Mana::Blue => self.blue_mana = self.blue_mana.saturating_add(1),
            Mana::Black => self.black_mana = self.black_mana.saturating_add(1),
            Mana::Red => self.red_mana = self.red_mana.saturating_add(1),
            Mana::Green => self.green_mana = self.green_mana.saturating_add(1),
            Mana::Colorless => self.colorless_mana = self.colorless_mana.saturating_add(1),
            Mana::Generic(count) => self.colorless_mana = self.colorless_mana.saturating_add(count),
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
                let copy = *self;

                for _ in 0..count {
                    let Some(mana) = self.max().checked_sub(1) else {
                        *self = copy;
                        return false;
                    };

                    *self.max() = mana;
                }
            }
        }

        true
    }

    fn max(&mut self) -> &mut usize {
        [
            &mut self.white_mana,
            &mut self.blue_mana,
            &mut self.black_mana,
            &mut self.red_mana,
            &mut self.green_mana,
            &mut self.colorless_mana,
        ]
        .into_iter()
        .max()
        .unwrap()
    }
}

impl Index<Owner> for AllPlayers {
    type Output = Player;

    fn index(&self, index: Owner) -> &Self::Output {
        self.players.get(&index).expect("Valid player id")
    }
}

impl IndexMut<Owner> for AllPlayers {
    fn index_mut(&mut self, index: Owner) -> &mut Self::Output {
        self.players.get_mut(&index).expect("Valid player id")
    }
}

impl Index<Controller> for AllPlayers {
    type Output = Player;

    fn index(&self, index: Controller) -> &Self::Output {
        self.players.get(&index.into()).expect("Valid player id")
    }
}

impl IndexMut<Controller> for AllPlayers {
    fn index_mut(&mut self, index: Controller) -> &mut Self::Output {
        self.players
            .get_mut(&index.into())
            .expect("Valid player id")
    }
}

#[derive(Debug, Default)]
pub struct AllPlayers {
    players: HashMap<Owner, Player>,
}

impl AllPlayers {
    #[must_use]
    pub fn new_player(&mut self) -> Owner {
        let id = Owner(NEXT_PLAYER_ID.fetch_add(1, Ordering::Relaxed));
        self.players.insert(
            id,
            Player {
                hexproof: false,
                lands_played: 0,
                mana_pool: Default::default(),
                deck: Deck::empty(),
            },
        );

        id
    }

    pub fn all_players(db: &mut Database) -> HashSet<Owner> {
        db.query::<&Owner>().iter(db).copied().collect()
    }
}

#[derive(Debug)]
pub struct Player {
    pub hexproof: bool,
    pub lands_played: usize,
    pub mana_pool: ManaPool,

    pub deck: Deck,
}

impl Player {
    #[cfg(test)]
    pub fn infinite_mana(&mut self) {
        self.mana_pool.white_mana = usize::MAX;
        self.mana_pool.blue_mana = usize::MAX;
        self.mana_pool.black_mana = usize::MAX;
        self.mana_pool.red_mana = usize::MAX;
        self.mana_pool.green_mana = usize::MAX;
        self.mana_pool.colorless_mana = usize::MAX;
    }

    pub fn draw_initial_hand(&mut self, db: &mut Database) {
        for _ in 0..7 {
            let card = self
                .deck
                .draw()
                .expect("Decks should have at least 7 cards");

            card.move_to_hand(db);
        }
    }

    pub fn draw(&mut self, db: &mut Database, count: usize) -> bool {
        if self.deck.len() < count {
            return false;
        }

        for _ in 0..count {
            let card = self.deck.draw().expect("Validated deck size");
            card.move_to_hand(db);
        }

        true
    }

    /// Returns Some if the card can be played
    pub fn play_card(
        &mut self,
        db: &mut Database,
        index: usize,
        target: Option<ActiveTarget>,
    ) -> anyhow::Result<Option<CardId>> {
        let cards = cards::<InHand>(db);
        let card = cards[index];
        let mana_pool = self.mana_pool;

        for mana in card.cost(db).mana_cost.iter() {
            if !self.mana_pool.spend(*mana) {
                self.mana_pool = mana_pool;
                return Ok(None);
            }
        }

        if let Some(target) = target {
            let targets = card.valid_targets(db);
            if !targets.contains(&target) {
                self.mana_pool = mana_pool;
                return Ok(None);
            }
        }

        if card.is_land(db) && self.lands_played >= Self::lands_per_turn(db) {
            return Ok(None);
        }

        Ok(Some(card))
    }

    pub fn spend_mana(&mut self, mana: &[Mana]) -> bool {
        let mana_pool = self.mana_pool;

        for mana in mana.iter().copied() {
            if !self.mana_pool.spend(mana) {
                self.mana_pool = mana_pool;
                return false;
            }
        }
        true
    }

    pub fn manifest(&mut self, db: &mut Database) -> Vec<UnresolvedActionResult> {
        if let Some(manifested) = self.deck.draw() {
            manifested.manifest(db);
            Battlefield::add_from_stack(db, manifested, vec![])
        } else {
            vec![]
        }
    }

    pub fn lands_per_turn(db: &mut Database) -> usize {
        1 + Battlefield::static_abilities(db)
            .into_iter()
            .filter_map(|(ability, _)| match ability {
                StaticAbility::ExtraLandsPerTurn(count) => Some(count),
                _ => None,
            })
            .sum::<usize>()
    }
}
