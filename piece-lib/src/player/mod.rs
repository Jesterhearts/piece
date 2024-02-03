pub(crate) mod mana_pool;

use std::ops::{Index, IndexMut};

use indexmap::IndexMap;
use itertools::Itertools;
use strum::IntoEnumIterator;
use uuid::Uuid;

use crate::{
    battlefield::Battlefields,
    effects::{ApplyResult, EffectBundle, PendingEffects},
    in_play::{CardId, Database},
    library::Library,
    log::{Log, LogEntry, LogId},
    player::mana_pool::ManaPool,
    protogen::{
        self,
        effects::{count::Fixed, Count, DrawCards, MoveToBattlefield},
        targets::{
            restriction::{self, EnteredBattlefieldThisTurn},
            Restriction,
        },
    },
    protogen::{
        effects::{static_ability, PopSelected},
        ids::UUID,
        mana::{spend_reason::Reason, Mana, ManaRestriction, ManaSource},
        targets::Location,
    },
    stack::{Selected, Stack, TargetType},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Owner(Uuid);

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

impl From<Owner> for protogen::ids::Owner {
    fn from(value: Owner) -> Self {
        let (hi, lo) = value.0.as_u64_pair();
        Self {
            id: protobuf::MessageField::some(UUID {
                hi,
                lo,
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl From<protogen::ids::Owner> for Owner {
    fn from(value: protogen::ids::Owner) -> Self {
        let hi = value.id.hi;
        let lo = value.id.lo;
        Self(Uuid::from_u64_pair(hi, lo))
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
            match restriction.restriction.as_ref().unwrap() {
                &restriction::Restriction::CanBeDamaged(_) => {}
                restriction::Restriction::AttackedThisTurn(_) => {
                    if db.turn.number_of_attackers_this_turn < 1 {
                        return false;
                    }
                }
                restriction::Restriction::Attacking(_) => {
                    return false;
                }
                restriction::Restriction::AttackingOrBlocking(_) => {
                    return false;
                }
                restriction::Restriction::CastFromHand(_) => {
                    return false;
                }
                restriction::Restriction::Chosen(_) => {
                    return false;
                }
                restriction::Restriction::Cmc(_) => {
                    return false;
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
                restriction::Restriction::ControllerJustCast(_) => {
                    if !Log::session(db, log_session).iter().any(|(_, entry)| {
                        if let LogEntry::Cast { card } = entry {
                            db[*card].controller == self
                        } else {
                            false
                        }
                    }) {
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
                        .get(&self)
                        .copied()
                        .unwrap_or_default();
                    if descended < 1 {
                        return false;
                    }
                }
                restriction::Restriction::DuringControllersTurn(_) => {
                    if self != db.turn.active_player() {
                        return false;
                    }
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
                            card.passes_restrictions(db, log_session, *card, restrictions)
                        })
                        .count() as i32;
                    if entered_this_turn < *count {
                        return false;
                    }
                }
                restriction::Restriction::HasActivatedAbility(_) => {
                    return false;
                }
                restriction::Restriction::InGraveyard(_) => {
                    return false;
                }
                restriction::Restriction::IsPermanent(_) => {
                    return false;
                }
                restriction::Restriction::IsPlayer(_) => {}
                restriction::Restriction::JustDiscarded(_) => {
                    return false;
                }
                restriction::Restriction::LifeGainedThisTurn(count) => {
                    let life_gained = db.all_players[self].life_gained_this_turn;
                    if life_gained < count.count {
                        return false;
                    }
                }
                restriction::Restriction::Location(_) => {
                    return false;
                }
                restriction::Restriction::ManaSpentFromSource(_) => {
                    return false;
                }
                restriction::Restriction::NonToken(_) => {
                    return false;
                }
                restriction::Restriction::NotChosen(_) => {
                    return false;
                }
                restriction::Restriction::NotKeywords(_) => {
                    return false;
                }
                restriction::Restriction::NotOfType(_) => {
                    return false;
                }
                restriction::Restriction::NotSelf(_) => {
                    if self == controller {
                        return false;
                    }
                }
                restriction::Restriction::NumberOfCountersOnThis(_) => {
                    /* TODO: Poison counters */
                    return false;
                }
                restriction::Restriction::OfColor(_) => {
                    return false;
                }
                restriction::Restriction::OfType(_) => {
                    return false;
                }
                restriction::Restriction::OnBattlefield(_) => {
                    return false;
                }
                restriction::Restriction::Power(_) => {
                    return false;
                }
                restriction::Restriction::Self_(_) => {
                    if self != controller {
                        return false;
                    }
                }
                restriction::Restriction::SourceCast(_) => {
                    return false;
                }
                restriction::Restriction::SpellOrAbilityJustCast(_) => {
                    return false;
                }
                restriction::Restriction::Tapped(_) => {
                    return false;
                }
                restriction::Restriction::TargetedBy(_) => {
                    /* TODO*/
                    return false;
                }
                restriction::Restriction::Threshold(_) => {
                    if db.graveyard[self].len() < 7 {
                        return false;
                    }
                }
                restriction::Restriction::Toughness(_) => {
                    return false;
                }
            }
        }

        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Controller(Uuid);

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
        let id = Owner(Uuid::new_v4());
        self.players.insert(
            id,
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
        self.players.keys().copied().collect_vec()
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

    pub fn draw_initial_hand(db: &mut Database, player: Owner) {
        for _ in 0..7 {
            let card = db.all_players[player]
                .library
                .draw()
                .expect("Decks should have at least 7 cards");

            card.move_to_hand(db);
        }
    }

    pub fn draw(player: Owner, count: u32) -> PendingEffects {
        let mut results = PendingEffects::default();
        results.push_back(EffectBundle {
            push_on_enter: Some(vec![Selected {
                location: None,
                target_type: TargetType::Player(player),
                targeted: false,
                restrictions: vec![],
            }]),
            effects: vec![
                DrawCards {
                    count: protobuf::MessageField::some(Count {
                        count: Some(
                            Fixed {
                                count: count as i32,
                                ..Default::default()
                            }
                            .into(),
                        ),
                        ..Default::default()
                    }),
                    ..Default::default()
                }
                .into(),
                PopSelected::default().into(),
            ],
            ..Default::default()
        });

        results
    }

    pub fn play_card(db: &mut Database, player: Owner, card: CardId) -> PendingEffects {
        assert!(db.hand[player].contains(&card));

        if card.is_land(db) && !Self::can_play_land(db, player) {
            return PendingEffects::default();
        }

        let mut db = scopeguard::guard(db, |db| db.stack.settle());
        if card.is_land(&db) {
            db.all_players[player].lands_played_this_turn += 1;
            return PendingEffects::from(EffectBundle {
                push_on_enter: Some(vec![Selected {
                    location: Some(Location::IN_HAND),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                }]),
                effects: vec![
                    MoveToBattlefield::default().into(),
                    PopSelected::default().into(),
                ],
                ..Default::default()
            });
        }

        Stack::move_card_to_stack_from_hand(&mut db, card)
    }

    pub(crate) fn pool_post_pay(
        &self,
        db: &Database,
        mana: &[Mana],
        sources: &[ManaSource],
        reason: &Reason,
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

    pub(crate) fn spend_mana(
        db: &mut Database,
        player: Owner,
        mana: &[Mana],
        sources: &[ManaSource],
        reason: &Reason,
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

    pub(crate) fn manifest(db: &mut Database, player: Owner) -> Option<ApplyResult> {
        if let Some(manifested) = db.all_players[player].library.draw() {
            db[manifested].manifested = true;
            db[manifested].facedown = true;
            Some(ApplyResult::PushBack(EffectBundle {
                push_on_enter: Some(vec![Selected {
                    location: Some(Location::IN_HAND),
                    target_type: TargetType::Card(manifested),
                    targeted: false,
                    restrictions: vec![],
                }]),
                effects: vec![
                    MoveToBattlefield::default().into(),
                    PopSelected::default().into(),
                ],
                ..Default::default()
            }))
        } else {
            None
        }
    }

    pub(crate) fn lands_per_turn(db: &mut Database, player: Owner) -> usize {
        1 + Battlefields::static_abilities(db)
            .into_iter()
            .filter_map(|(ability, card)| {
                if db[card].controller == player {
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

    pub fn can_play_land(db: &mut Database, player: Owner) -> bool {
        db.all_players[player].lands_played_this_turn < Self::lands_per_turn(db, player)
    }
}
