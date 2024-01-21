pub(crate) mod mana_pool;

use std::{
    ops::{Index, IndexMut},
    vec::IntoIter,
};

use indexmap::IndexMap;
use itertools::Itertools;
use strum::IntoEnumIterator;
use uuid::Uuid;

use crate::{
    action_result::ActionResult,
    battlefield::Battlefields,
    effects::EffectBehaviors,
    in_play::Database,
    library::Library,
    log::{Log, LogEntry, LogId},
    pending_results::PendingResults,
    player::mana_pool::{ManaPool, SpendReason},
    protogen::{
        cost::ManaCost,
        effects::{static_ability, ReplacementEffect},
        ids::{CardId, Controller, Owner},
        mana::{Mana, ManaRestriction, ManaSource},
        targets::Location,
    },
    protogen::{
        effects::replacement_effect::Replacing,
        targets::{
            restriction::{self, EnteredBattlefieldThisTurn},
            Restriction,
        },
    },
    stack::Stack,
};

impl From<Controller> for Owner {
    fn from(value: Controller) -> Self {
        Self {
            hi: value.hi,
            lo: value.lo,
            ..Default::default()
        }
    }
}

impl PartialEq<Controller> for Owner {
    fn eq(&self, other: &Controller) -> bool {
        self.hi == other.hi && self.lo == other.lo
    }
}

impl PartialEq<Owner> for Controller {
    fn eq(&self, other: &Owner) -> bool {
        self.hi == other.hi && self.lo == other.lo
    }
}

impl Owner {
    pub(crate) fn passes_restrictions(
        &self,
        db: &Database,
        log_session: LogId,
        controller: &Controller,
        restrictions: &[Restriction],
    ) -> bool {
        for restriction in restrictions {
            match restriction.restriction.as_ref().unwrap() {
                restriction::Restriction::AttackingOrBlocking(_) => {
                    return false;
                }
                restriction::Restriction::NotSelf(_) => {
                    if self == controller {
                        return false;
                    }
                }
                restriction::Restriction::Self_(_) => {
                    if self != controller {
                        return false;
                    }
                }
                restriction::Restriction::OfColor(_) => {
                    return false;
                }
                restriction::Restriction::OfType(_) => {
                    return false;
                }
                restriction::Restriction::NotOfType(_) => {
                    return false;
                }
                restriction::Restriction::CastFromHand(_) => {
                    return false;
                }
                restriction::Restriction::Cmc(_) => {
                    return false;
                }
                restriction::Restriction::Toughness(_) => {
                    return false;
                }
                restriction::Restriction::ControllerControlsColors(colors) => {
                    let controlled_colors = Battlefields::controlled_colors(db, controller);
                    if !colors
                        .colors
                        .iter()
                        .any(|color| controlled_colors.contains(&color.enum_value().unwrap()))
                    {
                        return false;
                    }
                }
                restriction::Restriction::ControllerHandEmpty(_) => {
                    if controller.has_cards(db, Location::IN_HAND) {
                        return false;
                    }
                }
                restriction::Restriction::InGraveyard(_) => {
                    return false;
                }
                restriction::Restriction::OnBattlefield(_) => {
                    return false;
                }
                restriction::Restriction::Location(_) => {
                    return false;
                }
                restriction::Restriction::Attacking(_) => {
                    return false;
                }
                restriction::Restriction::NotKeywords(_) => {
                    return false;
                }
                restriction::Restriction::LifeGainedThisTurn(count) => {
                    let life_gained = db.all_players[self].life_gained_this_turn;
                    if life_gained < count.count {
                        return false;
                    }
                }
                restriction::Restriction::Descend(count) => {
                    let cards = db.graveyard[self]
                        .iter()
                        .filter(|card| card.is_permanent(db))
                        .count() as i32;
                    if cards < count.count {
                        return false;
                    }
                }
                restriction::Restriction::DescendedThisTurn(_) => {
                    let descended = db
                        .graveyard
                        .descended_this_turn
                        .get(self)
                        .copied()
                        .unwrap_or_default();
                    if descended < 1 {
                        return false;
                    }
                }
                restriction::Restriction::Tapped(_) => {
                    return false;
                }
                restriction::Restriction::ManaSpentFromSource(_) => {
                    return false;
                }
                restriction::Restriction::Power(_) => {
                    return false;
                }
                restriction::Restriction::NotChosen(_) => {
                    return false;
                }
                restriction::Restriction::SourceCast(_) => {
                    return false;
                }
                restriction::Restriction::DuringControllersTurn(_) => {
                    if *self != db.turn.active_player() {
                        return false;
                    }
                }
                restriction::Restriction::ControllerJustCast(_) => {
                    if !Log::session(db, log_session).iter().any(|(_, entry)| {
                        if let LogEntry::Cast { card } = entry {
                            db[card].controller == *self
                        } else {
                            false
                        }
                    }) {
                        return false;
                    }
                }
                restriction::Restriction::Controller(controller_restriction) => {
                    match controller_restriction.controller.as_ref().unwrap() {
                        restriction::controller::Controller::Self_(_) => {
                            if self != controller {
                                return false;
                            }
                        }
                        restriction::controller::Controller::Opponent(_) => {
                            if self == controller {
                                return false;
                            }
                        }
                    }
                }
                restriction::Restriction::NumberOfCountersOnThis(_) => {
                    // TODO: Poison counters
                    return false;
                }
                restriction::Restriction::EnteredBattlefieldThisTurn(
                    EnteredBattlefieldThisTurn {
                        count,
                        restrictions,
                        ..
                    },
                ) => {
                    let entered_this_turn = CardId::entered_battlefield_this_turn(db)
                        .filter(|card| {
                            card.passes_restrictions(db, log_session, card, restrictions)
                        })
                        .count() as i32;
                    if entered_this_turn < *count {
                        return false;
                    }
                }
                restriction::Restriction::AttackedThisTurn(_) => {
                    if db.turn.number_of_attackers_this_turn < 1 {
                        return false;
                    }
                }
                restriction::Restriction::Threshold(_) => {
                    if db.graveyard[self].len() < 7 {
                        return false;
                    }
                }
                restriction::Restriction::NonToken(_) => {
                    return false;
                }
                restriction::Restriction::TargetedBy(_) => {
                    // TODO
                    return false;
                }
                restriction::Restriction::HasActivatedAbility(_) => {
                    return false;
                }
                restriction::Restriction::SpellOrAbilityJustCast(_) => {
                    return false;
                }
                restriction::Restriction::IsPermanent(_) => {
                    return false;
                }
                restriction::Restriction::Chosen(_) => {
                    return false;
                }
                restriction::Restriction::JustDiscarded(_) => {
                    return false;
                }
            }
        }

        true
    }
}

impl From<Owner> for Controller {
    fn from(value: Owner) -> Self {
        Self {
            hi: value.hi,
            lo: value.lo,
            ..Default::default()
        }
    }
}

impl Controller {
    pub(crate) fn has_cards(&self, db: &Database, location: Location) -> bool {
        match location {
            Location::ON_BATTLEFIELD => !db.battlefield[self].is_empty(),
            Location::IN_GRAVEYARD => !db.graveyard[self].is_empty(),
            Location::IN_EXILE => !db.exile[self].is_empty(),
            Location::IN_LIBRARY => !db.all_players[self].library.is_empty(),
            Location::IN_HAND => !db.hand[self].is_empty(),
            Location::IN_STACK => unreachable!(),
        }
    }
}

impl Index<&Owner> for AllPlayers {
    type Output = Player;

    fn index(&self, index: &Owner) -> &Self::Output {
        self.players.get(index).expect("Invalid player id")
    }
}

impl IndexMut<&Owner> for AllPlayers {
    fn index_mut(&mut self, index: &Owner) -> &mut Self::Output {
        self.players.get_mut(index).expect("Invalid player id")
    }
}

impl Index<&Controller> for AllPlayers {
    type Output = Player;

    fn index(&self, index: &Controller) -> &Self::Output {
        self.players
            .get(&Owner::from(index.clone()))
            .expect("Invalid player id")
    }
}

impl IndexMut<&Controller> for AllPlayers {
    fn index_mut(&mut self, index: &Controller) -> &mut Self::Output {
        self.players
            .get_mut(&Owner::from(index.clone()))
            .expect("Invalid player id")
    }
}

#[derive(Debug, Default)]
pub struct AllPlayers {
    players: IndexMap<Owner, Player>,
}

impl AllPlayers {
    #[must_use]
    pub fn new_player(&mut self, name: String, life_total: i32) -> Owner {
        let (hi, lo) = Uuid::new_v4().as_u64_pair();
        let id = Owner {
            hi,
            lo,
            ..Default::default()
        };
        self.players.insert(
            id.clone(),
            Player {
                name,
                hexproof: false,
                life_total,
                hand_size: 7,
                lands_played_this_turn: 0,
                life_gained_this_turn: 0,
                ban_attacking_this_turn: false,
                mana_pool: Default::default(),
                library: Library::empty(),
                lost: false,
            },
        );

        id
    }

    pub(crate) fn all_players(&self) -> Vec<Owner> {
        self.players.keys().cloned().collect_vec()
    }
}

#[derive(Debug)]
pub struct Player {
    pub name: String,

    #[allow(unused)]
    pub(crate) hexproof: bool,
    pub(crate) hand_size: usize,
    pub mana_pool: ManaPool,

    pub(crate) lands_played_this_turn: usize,
    pub(crate) ban_attacking_this_turn: bool,
    pub(crate) life_gained_this_turn: u32,

    pub life_total: i32,

    pub library: Library,

    pub lost: bool,
}

impl Player {
    pub fn infinite_mana(&mut self) {
        for mana in Mana::iter() {
            *self
                .mana_pool
                .sourced
                .entry(mana)
                .or_default()
                .entry(ManaSource::ANY)
                .or_default()
                .entry(ManaRestriction::NONE)
                .or_default() = usize::MAX;
        }
    }

    pub fn draw_initial_hand(db: &mut Database, player: &Owner) {
        for _ in 0..7 {
            let card = db.all_players[player]
                .library
                .draw()
                .expect("Decks should have at least 7 cards");

            card.move_to_hand(db);
        }
    }

    pub fn draw(db: &mut Database, player: &Owner, count: usize) -> PendingResults {
        let mut results = PendingResults::default();

        for _ in 0..count {
            let replacements = db.replacement_abilities_watching(Replacing::DRAW);
            if !replacements.is_empty() {
                Self::draw_with_replacement(
                    db,
                    player,
                    &mut replacements.into_iter(),
                    1,
                    &mut results,
                );
            } else if let Some(card) = db.all_players[player].library.draw() {
                card.move_to_hand(db);
            } else {
                results.push_settled(ActionResult::PlayerLoses(player.clone()));
                return results;
            }
        }

        results
    }

    pub(crate) fn draw_with_replacement(
        db: &mut Database,
        player: &Owner,
        replacements: &mut IntoIter<(CardId, ReplacementEffect)>,
        count: usize,
        results: &mut PendingResults,
    ) {
        for _ in 0..count {
            if replacements.len() > 0 {
                while let Some((source, replacement)) = replacements.next() {
                    if !source.passes_restrictions(
                        db,
                        LogId::current(db),
                        &source,
                        &replacement.restrictions,
                    ) {
                        continue;
                    }

                    let controller = db[&source].controller.clone();
                    for effect in replacement.effects.iter() {
                        effect.effect.as_ref().unwrap().replace_draw(
                            db,
                            player,
                            replacements,
                            &controller,
                            count,
                            results,
                        );
                    }
                }
            } else if let Some(card) = db.all_players[player].library.draw() {
                card.move_to_hand(db);
            } else {
                return;
            }
        }
    }

    pub fn play_card(db: &mut Database, player: &Owner, card: &CardId) -> PendingResults {
        assert!(db.hand[player].contains(card));

        if card.is_land(db) && !Self::can_play_land(db, player) {
            return PendingResults::default();
        }

        let mut db = scopeguard::guard(db, |db| db.stack.settle());
        if card.is_land(&db) {
            db.all_players[player].lands_played_this_turn += 1;
            return Battlefields::add_from_stack_or_hand(&mut db, card, None);
        }

        Stack::move_card_to_stack_from_hand(&mut db, card.clone(), true)
    }

    pub(crate) fn can_meet_cost(
        &self,
        db: &Database,
        mana: &[protobuf::EnumOrUnknown<ManaCost>],
        sources: &[ManaSource],
        reason: &SpendReason,
    ) -> bool {
        for (cost, source) in mana.iter().copied().zip(
            sources
                .iter()
                .copied()
                .chain(std::iter::repeat(ManaSource::ANY)),
        ) {
            if !self
                .mana_pool
                .can_spend(db, cost.enum_value().unwrap(), source, reason)
            {
                return false;
            }
        }

        true
    }

    pub(crate) fn pool_post_pay(
        &self,
        db: &Database,
        mana: &[Mana],
        sources: &[ManaSource],
        reason: &SpendReason,
    ) -> Option<ManaPool> {
        let mut mana_pool = self.mana_pool.clone();

        for (mana, source) in mana.iter().copied().zip(
            sources
                .iter()
                .copied()
                .chain(std::iter::repeat(ManaSource::ANY)),
        ) {
            if let (false, _) = mana_pool.spend(db, mana, source, reason) {
                return None;
            }
        }

        Some(mana_pool)
    }

    pub(crate) fn can_spend_mana(
        &self,
        db: &Database,
        mana: &[Mana],
        sources: &[ManaSource],
        reason: &SpendReason,
    ) -> bool {
        self.pool_post_pay(db, mana, sources, reason).is_some()
    }

    pub(crate) fn spend_mana(
        db: &mut Database,
        player: &Owner,
        mana: &[Mana],
        sources: &[ManaSource],
        reason: &SpendReason,
    ) -> bool {
        let mut mana_pool = db.all_players[player].mana_pool.clone();

        for (mana, source) in mana.iter().copied().zip(
            sources
                .iter()
                .copied()
                .chain(std::iter::repeat(ManaSource::ANY)),
        ) {
            if let (false, _) = mana_pool.spend(db, mana, source, reason) {
                return false;
            }
        }

        db.all_players[player].mana_pool = mana_pool;
        true
    }

    pub(crate) fn manifest(db: &mut Database, player: &Owner) -> PendingResults {
        if let Some(manifested) = db.all_players[player].library.draw() {
            {
                db[&manifested].manifested = true;
                db[&manifested].facedown = true;
            };
            Battlefields::add_from_stack_or_hand(db, &manifested, None)
        } else {
            PendingResults::default()
        }
    }

    pub(crate) fn lands_per_turn(db: &mut Database, player: &Owner) -> usize {
        1 + Battlefields::static_abilities(db)
            .into_iter()
            .filter_map(|(ability, card)| {
                if db[card].controller == *player {
                    match ability {
                        static_ability::Ability::ExtraLandsPerTurn(count) => {
                            Some(count.count as usize)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .sum::<usize>()
    }

    pub fn can_play_land(db: &mut Database, player: &Owner) -> bool {
        db.all_players[player].lands_played_this_turn < Self::lands_per_turn(db, player)
    }
}
