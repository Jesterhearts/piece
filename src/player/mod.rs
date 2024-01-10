pub(crate) mod mana_pool;

use std::{
    ops::{Index, IndexMut},
    sync::atomic::{AtomicUsize, Ordering},
    vec::IntoIter,
};

use indexmap::IndexMap;
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    abilities::StaticAbility,
    battlefield::{ActionResult, Battlefields},
    effects::{EffectBehaviors, ReplacementAbility, Replacing},
    in_play::{CardId, Database},
    library::Library,
    log::{Log, LogEntry, LogId},
    pending_results::PendingResults,
    player::mana_pool::{ManaPool, SpendReason},
    protogen::{
        color::Color,
        cost::ManaCost,
        mana::{Mana, ManaRestriction},
        targets::{Location, ManaSource},
    },
    stack::Stack,
    targets::Restriction,
};

static NEXT_PLAYER_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
    pub(crate) fn passes_restrictions(
        self,
        db: &Database,
        log_session: LogId,
        controller: Controller,
        restrictions: &[Restriction],
    ) -> bool {
        for restriction in restrictions {
            match restriction {
                Restriction::AttackingOrBlocking => {
                    return false;
                }
                Restriction::NotSelf => {
                    if self == controller {
                        return false;
                    }
                }
                Restriction::Self_ => {
                    if self != controller {
                        return false;
                    }
                }
                Restriction::OfColor(_) => {
                    return false;
                }
                Restriction::OfType { .. } => {
                    return false;
                }
                Restriction::NotOfType { .. } => {
                    return false;
                }
                Restriction::CastFromHand => {
                    return false;
                }
                Restriction::Cmc(_) => {
                    return false;
                }
                Restriction::Toughness(_) => {
                    return false;
                }
                Restriction::ControllerControlsBlackOrGreen => {
                    let colors = Battlefields::controlled_colors(db, controller);
                    if !(colors.contains(&Color::GREEN) || colors.contains(&Color::BLACK)) {
                        return false;
                    }
                }
                Restriction::ControllerHandEmpty => {
                    if controller.has_cards(db, Location::IN_HAND) {
                        return false;
                    }
                }
                Restriction::InGraveyard => {
                    return false;
                }
                Restriction::OnBattlefield => {
                    return false;
                }
                Restriction::InLocation { .. } => {
                    return false;
                }
                Restriction::Attacking => {
                    return false;
                }
                Restriction::NotKeywords(_) => {
                    return false;
                }
                Restriction::LifeGainedThisTurn(count) => {
                    let life_gained = db
                        .turn
                        .life_gained_this_turn
                        .get(&self)
                        .copied()
                        .unwrap_or_default();
                    if life_gained < *count {
                        return false;
                    }
                }
                Restriction::Descend(count) => {
                    let cards = db.graveyard[self]
                        .iter()
                        .filter(|card| card.is_permanent(db))
                        .count();
                    if cards < *count {
                        return false;
                    }
                }
                Restriction::DescendedThisTurn => {
                    let descended = db
                        .graveyard
                        .descended_this_turn
                        .get(&self)
                        .copied()
                        .unwrap_or_default();
                    if descended < 1 {
                        return false;
                    }
                }
                Restriction::Tapped => {
                    return false;
                }
                Restriction::ManaSpentFromSource(_) => {
                    return false;
                }
                Restriction::Power(_) => {
                    return false;
                }
                Restriction::NotChosen => {
                    return false;
                }
                Restriction::SourceCast => {
                    return false;
                }
                Restriction::DuringControllersTurn => {
                    if self != db.turn.active_player() {
                        return false;
                    }
                }
                Restriction::ControllerJustCast => {
                    if !Log::current_session(db).iter().any(|(_, entry)| {
                        if let LogEntry::Cast { card } = entry {
                            db[*card].controller == self
                        } else {
                            false
                        }
                    }) {
                        return false;
                    }
                }
                Restriction::Controller(controller_restriction) => match controller_restriction {
                    crate::targets::ControllerRestriction::Self_ => {
                        if self != controller {
                            return false;
                        }
                    }
                    crate::targets::ControllerRestriction::Opponent => {
                        if self == controller {
                            return false;
                        }
                    }
                },
                Restriction::NumberOfCountersOnThis { .. } => {
                    // TODO: Poison counters
                    return false;
                }
                Restriction::EnteredTheBattlefieldThisTurn {
                    count,
                    restrictions,
                } => {
                    let entered_this_turn = CardId::entered_battlefield_this_turn(db)
                        .filter(|card| {
                            card.passes_restrictions(db, log_session, *card, restrictions)
                        })
                        .count();
                    if entered_this_turn < *count {
                        return false;
                    }
                }
                Restriction::AttackedThisTurn => {
                    if db.turn.number_of_attackers_this_turn < 1 {
                        return false;
                    }
                }
                Restriction::Threshold => {
                    if db.graveyard[self].len() < 7 {
                        return false;
                    }
                }
                Restriction::NonToken => {
                    return false;
                }
                Restriction::TargetedBy => {
                    // TODO
                    return false;
                }
                Restriction::HasActivatedAbility => {
                    return false;
                }
                Restriction::SpellOrAbilityJustCast => {
                    return false;
                }
            }
        }

        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Controller(usize);

impl From<Owner> for Controller {
    fn from(value: Owner) -> Self {
        Self(value.0)
    }
}

impl Controller {
    pub(crate) fn has_cards(self, db: &Database, location: Location) -> bool {
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

impl Index<Owner> for AllPlayers {
    type Output = Player;

    fn index(&self, index: Owner) -> &Self::Output {
        self.players.get(&index).expect("Invalid player id")
    }
}

impl IndexMut<Owner> for AllPlayers {
    fn index_mut(&mut self, index: Owner) -> &mut Self::Output {
        self.players.get_mut(&index).expect("Invalid player id")
    }
}

impl Index<Controller> for AllPlayers {
    type Output = Player;

    fn index(&self, index: Controller) -> &Self::Output {
        self.players
            .get(&Owner::from(index))
            .expect("Invalid player id")
    }
}

impl IndexMut<Controller> for AllPlayers {
    fn index_mut(&mut self, index: Controller) -> &mut Self::Output {
        self.players
            .get_mut(&Owner::from(index))
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
        let id = Owner(NEXT_PLAYER_ID.fetch_add(1, Ordering::Relaxed));
        self.players.insert(
            id,
            Player {
                name,
                hexproof: false,
                life_total,
                lands_played: 0,
                hand_size: 7,
                mana_pool: Default::default(),
                library: Library::empty(),
                lost: false,
            },
        );

        id
    }

    pub(crate) fn all_players(&self) -> Vec<Owner> {
        self.players.keys().copied().collect_vec()
    }
}

#[derive(Debug)]
pub struct Player {
    pub name: String,

    #[allow(unused)]
    pub(crate) hexproof: bool,
    pub(crate) lands_played: usize,
    pub(crate) hand_size: usize,
    pub mana_pool: ManaPool,

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

    pub fn draw_initial_hand(&mut self, db: &mut Database) {
        for _ in 0..7 {
            let card = self
                .library
                .draw()
                .expect("Decks should have at least 7 cards");

            card.move_to_hand(db);
        }
    }

    pub fn draw(db: &mut Database, player: Owner, count: usize) -> PendingResults {
        let mut results = PendingResults::default();

        for _ in 0..count {
            let replacements = db.replacement_abilities_watching(Replacing::Draw);
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
                results.push_settled(ActionResult::PlayerLoses(player));
                return results;
            }
        }

        results
    }

    pub(crate) fn draw_with_replacement(
        db: &mut Database,
        player: Owner,
        replacements: &mut IntoIter<(CardId, ReplacementAbility)>,
        count: usize,
        results: &mut PendingResults,
    ) {
        for _ in 0..count {
            if replacements.len() > 0 {
                while let Some((source, replacement)) = replacements.next() {
                    if !source.passes_restrictions(
                        db,
                        LogId::current(db),
                        source,
                        &replacement.restrictions,
                    ) {
                        continue;
                    }

                    for effect in replacement.effects.iter() {
                        effect.effect.replace_draw(
                            db,
                            player,
                            replacements,
                            db[source].controller,
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

    pub fn play_card(db: &mut Database, player: Owner, card: CardId) -> PendingResults {
        assert!(db.hand[player].contains(&card));

        if card.is_land(db)
            && db.all_players[player].lands_played >= Self::lands_per_turn(db, player)
        {
            return PendingResults::default();
        }

        let mut db = scopeguard::guard(db, |db| db.stack.settle());
        if card.is_land(&db) {
            db.all_players[player].lands_played += 1;
            return Battlefields::add_from_stack_or_hand(&mut db, card, None);
        }

        Stack::move_card_to_stack_from_hand(&mut db, card, true)
    }

    pub(crate) fn can_meet_cost(
        &self,
        db: &Database,
        mana: &[protobuf::EnumOrUnknown<ManaCost>],
        sources: &[ManaSource],
        reason: SpendReason,
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
        reason: SpendReason,
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
        reason: SpendReason,
    ) -> bool {
        self.pool_post_pay(db, mana, sources, reason).is_some()
    }

    pub(crate) fn spend_mana(
        db: &mut Database,
        player: Owner,
        mana: &[Mana],
        sources: &[ManaSource],
        reason: SpendReason,
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

    pub(crate) fn manifest(db: &mut Database, player: Owner) -> PendingResults {
        if let Some(manifested) = db.all_players[player].library.draw() {
            {
                db[manifested].manifested = true;
                db[manifested].facedown = true;
            };
            Battlefields::add_from_stack_or_hand(db, manifested, None)
        } else {
            PendingResults::default()
        }
    }

    pub(crate) fn lands_per_turn(db: &mut Database, player: Owner) -> usize {
        1 + Battlefields::static_abilities(db)
            .into_iter()
            .filter_map(|(ability, card)| {
                if db[card].controller == player {
                    match ability {
                        StaticAbility::ExtraLandsPerTurn(count) => Some(count),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .sum::<usize>()
    }
}
