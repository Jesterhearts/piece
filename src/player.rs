use std::{
    collections::HashSet,
    ops::{Index, IndexMut},
    sync::atomic::{AtomicUsize, Ordering},
};

use bevy_ecs::{component::Component, entity::Entity, query::With};
use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    abilities::StaticAbility,
    battlefield::{ActionResult, Battlefield, PendingResults},
    controller::ControllerRestriction,
    deck::Deck,
    effects::{replacing, Effect},
    in_play::{cards, CardId, Database, InHand, ReplacementEffectId},
    mana::{Mana, ManaCost},
    stack::Stack,
};

static NEXT_PLAYER_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct Owner(usize);

impl From<Controller> for Owner {
    fn from(value: Controller) -> Self {
        Self(value.0)
    }
}

impl PartialEq<Controller> for Owner {
    fn eq(&self, other: &Controller) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<Owner> for Controller {
    fn eq(&self, other: &Owner) -> bool {
        self.0 == other.0
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

    pub fn name(self, _db: &Database) -> String {
        "Player".to_string()
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

    pub fn has_cards<Zone: Component>(self, db: &mut Database) -> bool {
        db.query_filtered::<&Controller, With<Zone>>()
            .iter(db)
            .any(|owner| self == *owner)
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
        }
    }

    pub fn spend(&mut self, mana: Mana) -> bool {
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
        }

        true
    }

    pub fn can_spend(&self, cost: ManaCost) -> bool {
        let mut mana_pool = *self;
        match cost {
            ManaCost::White => {
                if !mana_pool.spend(Mana::White) {
                    return false;
                }
            }
            ManaCost::Blue => {
                if !mana_pool.spend(Mana::Blue) {
                    return false;
                }
            }
            ManaCost::Black => {
                if !mana_pool.spend(Mana::Black) {
                    return false;
                }
            }
            ManaCost::Red => {
                if !mana_pool.spend(Mana::Red) {
                    return false;
                }
            }
            ManaCost::Green => {
                if !mana_pool.spend(Mana::Green) {
                    return false;
                }
            }
            ManaCost::Colorless => {
                if !mana_pool.spend(Mana::Colorless) {
                    return false;
                }
            }
            ManaCost::Generic(count) => {
                for _ in 0..count {
                    if let Some(max) = mana_pool.max() {
                        if !mana_pool.spend(max) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
            ManaCost::X => {}
        }

        true
    }

    pub fn available_mana(&self) -> impl Iterator<Item = (usize, Mana)> {
        [
            (self.white_mana, Mana::White),
            (self.blue_mana, Mana::Blue),
            (self.black_mana, Mana::Black),
            (self.red_mana, Mana::Red),
            (self.green_mana, Mana::Green),
            (self.colorless_mana, Mana::Colorless),
        ]
        .into_iter()
    }

    pub fn max(&self) -> Option<Mana> {
        self.available_mana()
            .max_by_key(|(count, _)| *count)
            .filter(|(count, _)| *count > 0)
            .map(|(_, mana)| mana)
    }

    pub fn pools_display(&self) -> Vec<String> {
        let symbols = [
            Mana::White,
            Mana::Blue,
            Mana::Black,
            Mana::Red,
            Mana::Green,
            Mana::Colorless,
        ];
        let pools = [
            &self.white_mana,
            &self.blue_mana,
            &self.black_mana,
            &self.red_mana,
            &self.green_mana,
            &self.colorless_mana,
        ];

        let mut results = vec![];
        for (symbol, amount) in symbols.into_iter().zip(pools) {
            let mut result = String::default();
            symbol.push_mana_symbol(&mut result);
            result.push_str(&format!(": {}", amount));
            results.push(result)
        }

        results
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
        self.players
            .get(&Owner::from(index))
            .expect("Valid player id")
    }
}

impl IndexMut<Controller> for AllPlayers {
    fn index_mut(&mut self, index: Controller) -> &mut Self::Output {
        self.players
            .get_mut(&Owner::from(index))
            .expect("Valid player id")
    }
}

#[derive(Debug, Default)]
pub struct AllPlayers {
    players: IndexMap<Owner, Player>,
}

impl AllPlayers {
    #[must_use]
    pub fn new_player(&mut self, name: String, life_total: i32) -> Owner {
        let id = Owner(NEXT_PLAYER_ID.fetch_add(1, Ordering::Relaxed));
        self.players.insert(
            id,
            Player {
                name,
                hexproof: false,
                life_total,
                lands_played: 0,
                mana_pool: Default::default(),
                deck: Deck::empty(),
                lost: false,
            },
        );

        id
    }

    pub fn all_players(&self) -> Vec<Owner> {
        self.players.keys().copied().collect_vec()
    }

    pub fn all_players_in_db(db: &mut Database) -> HashSet<Owner> {
        db.query::<&Owner>().iter(db).copied().collect()
    }
}

#[derive(Debug)]
pub struct Player {
    pub name: String,

    pub hexproof: bool,
    pub lands_played: usize,
    pub mana_pool: ManaPool,

    pub life_total: i32,

    pub deck: Deck,

    pub lost: bool,
}

impl Player {
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

    pub fn draw(&mut self, db: &mut Database, count: usize) -> PendingResults {
        let mut results = PendingResults::default();

        for _ in 0..count {
            let replacements = ReplacementEffectId::watching::<replacing::Draw>(db);
            if !replacements.is_empty() {
                self.draw_internal(db, &mut replacements.into_iter(), 1, &mut results);
            } else if let Some(card) = self.deck.draw() {
                card.move_to_hand(db);
            } else {
                return results;
            }
        }

        results
    }

    fn draw_internal(
        &mut self,
        db: &mut Database,
        replacements: &mut impl ExactSizeIterator<Item = ReplacementEffectId>,
        count: usize,
        results: &mut PendingResults,
    ) {
        for _ in 0..count {
            if replacements.len() > 0 {
                while let Some(replacement) = replacements.next() {
                    let source = replacement.source(db);
                    let restrictions = replacement.restrictions(db);
                    if !source.passes_restrictions(
                        db,
                        source,
                        ControllerRestriction::Any,
                        &restrictions,
                    ) {
                        continue;
                    }

                    let controller = replacement.source(db).controller(db);
                    for effect in replacement.effects(db) {
                        match effect.into_effect(db, controller) {
                            Effect::BattlefieldModifier(_) => todo!(),
                            Effect::ControllerDrawCards(count) => {
                                self.draw_internal(db, replacements, count, results);
                            }
                            Effect::ControllerLosesLife(count) => {
                                results.push_settled(ActionResult::LoseLife {
                                    target: controller,
                                    count,
                                });
                            }
                            _ => todo!(),
                        }
                    }
                }
            } else if let Some(card) = self.deck.draw() {
                card.move_to_hand(db);
            } else {
                return;
            }
        }
    }

    pub fn play_card(&mut self, db: &mut Database, index: usize) -> PendingResults {
        let cards = cards::<InHand>(db);
        let card = cards[index];

        if card.is_land(db) && self.lands_played >= Self::lands_per_turn(db) {
            return PendingResults::default();
        }

        let mut db = scopeguard::guard(db, Stack::settle);
        if card.is_land(&db) {
            self.lands_played += 1;
            return Battlefield::add_from_stack_or_hand(&mut db, card, None);
        }

        Stack::move_card_to_stack_from_hand(&mut db, card, true)
    }

    pub fn can_meet_cost(&self, mana: &[ManaCost]) -> bool {
        let mut mana = mana.to_vec();
        mana.sort();

        for cost in mana.iter().copied() {
            if !self.mana_pool.can_spend(cost) {
                return false;
            }
        }

        true
    }

    pub fn pool_post_pay(&self, mana: &[Mana]) -> Option<ManaPool> {
        let mut mana_pool = self.mana_pool;

        for mana in mana.iter().copied() {
            if !mana_pool.spend(mana) {
                return None;
            }
        }

        Some(mana_pool)
    }

    pub fn can_spend_mana(&self, mana: &[Mana]) -> bool {
        self.pool_post_pay(mana).is_some()
    }

    pub fn spend_mana(&mut self, mana: &[Mana]) -> bool {
        let mut mana_pool = self.mana_pool;

        for mana in mana.iter().copied() {
            if !mana_pool.spend(mana) {
                return false;
            }
        }

        self.mana_pool = mana_pool;
        true
    }

    pub fn manifest(&mut self, db: &mut Database) -> PendingResults {
        if let Some(manifested) = self.deck.draw() {
            manifested.manifest(db);
            Battlefield::add_from_stack_or_hand(db, manifested, None)
        } else {
            PendingResults::default()
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
