use std::{
    collections::{HashMap, HashSet},
    ops::{Index, IndexMut},
    sync::atomic::{AtomicUsize, Ordering},
};

use indoc::indoc;
use rusqlite::{types::FromSql, Connection, ToSql};

use crate::{
    abilities::StaticAbility,
    battlefield::{Battlefield, UnresolvedActionResult},
    deck::Deck,
    in_play::{CardId, Location},
    mana::Mana,
    stack::ActiveTarget,
};

static NEXT_PLAYER_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct PlayerId(usize);

impl PlayerId {
    fn new() -> Self {
        Self(NEXT_PLAYER_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub(crate) fn get_cards_in_zone(
        &self,
        db: &Connection,
        location: Location,
    ) -> anyhow::Result<Vec<CardId>> {
        let mut results = vec![];
        let mut in_location = db.prepare(indoc! {"
            SELECT cardid
            FROM cards
            WHERE controller = (?1) AND location = (?2)
            ORDER BY location_seq ASC
        "})?;
        for row in
            in_location.query_map((self, serde_json::to_string(&location)?), |row| row.get(0))?
        {
            results.push(row?)
        }

        Ok(results)
    }
}

impl FromSql for PlayerId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self(usize::column_result(value)?))
    }
}

impl ToSql for PlayerId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
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

impl Index<PlayerId> for AllPlayers {
    type Output = Player;

    fn index(&self, index: PlayerId) -> &Self::Output {
        self.players.get(&index).expect("Valid player id")
    }
}

impl IndexMut<PlayerId> for AllPlayers {
    fn index_mut(&mut self, index: PlayerId) -> &mut Self::Output {
        self.players.get_mut(&index).expect("Valid player id")
    }
}

#[derive(Debug, Default)]
pub struct AllPlayers {
    players: HashMap<PlayerId, Player>,
}

impl AllPlayers {
    #[must_use]
    pub fn new_player(&mut self) -> PlayerId {
        let id = PlayerId::new();
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

    pub fn all_players(db: &Connection) -> anyhow::Result<HashSet<PlayerId>> {
        let mut result = HashSet::default();
        let mut owners = db.prepare("SELECT DISTINCT owner FROM cards")?;
        for row in owners.query_map((), |row| row.get(0))? {
            result.insert(row?);
        }

        Ok(result)
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

    pub fn draw_initial_hand(&mut self, db: &Connection) -> anyhow::Result<()> {
        for _ in 0..7 {
            let card = self
                .deck
                .draw()
                .expect("Decks should have at least 7 cards");

            card.move_to_hand(db)?;
        }

        Ok(())
    }

    pub fn draw(&mut self, db: &Connection, count: usize) -> anyhow::Result<bool> {
        if self.deck.len() < count {
            return Ok(false);
        }

        for _ in 0..count {
            let card = self.deck.draw().expect("Validated deck size");
            card.move_to_hand(db)?;
        }

        Ok(true)
    }

    /// Returns Some if the card can be played
    pub fn play_card(
        &mut self,
        db: &Connection,
        index: usize,
        target: Option<ActiveTarget>,
    ) -> anyhow::Result<Option<CardId>> {
        let cards = Location::Hand.cards_in(db)?;
        let card = cards[index];
        let mana_pool = self.mana_pool;

        for mana in card.cost(db)?.mana_cost.into_iter() {
            if !self.mana_pool.spend(mana) {
                self.mana_pool = mana_pool;
                return Ok(None);
            }
        }

        if let Some(target) = target {
            let targets = card.valid_targets(db)?;
            if !targets.contains(&target) {
                self.mana_pool = mana_pool;
                return Ok(None);
            }
        }

        if card.requires_target(db)? && target.is_none() {
            return Ok(None);
        }

        if card.is_land(db)? && self.lands_played >= Self::lands_per_turn(db)? {
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

    pub fn manifest(&mut self, db: &Connection) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        if let Some(manifested) = self.deck.draw() {
            manifested.manifest(db)?;
            Battlefield::add(db, manifested, vec![])
        } else {
            Ok(vec![])
        }
    }

    pub fn lands_per_turn(db: &Connection) -> Result<usize, anyhow::Error> {
        Ok(1 + Battlefield::static_abilities(db)?
            .into_iter()
            .filter_map(|(ability, _)| match ability {
                StaticAbility::ExtraLandsPerTurn(count) => Some(count),
                _ => None,
            })
            .sum::<usize>())
    }
}
