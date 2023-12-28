mod abilityid;
mod auraid;
mod cardid;
mod counterid;
mod modifierid;
mod replacementid;
mod triggerid;

use std::{
    collections::{HashMap, HashSet},
    ops::Neg,
    sync::atomic::{AtomicUsize, Ordering},
};

use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, Events},
    system::Resource,
    world::World,
};
use derive_more::{Deref, DerefMut};
use itertools::Itertools;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Event)]
pub(crate) struct DeleteAbility {
    pub(crate) ability: AbilityId,
}

pub use abilityid::AbilityId;
pub(crate) use auraid::AuraId;
pub(crate) use cardid::target_from_location;
pub use cardid::CardId;
pub(crate) use counterid::CounterId;
pub(crate) use modifierid::{ModifierId, ModifierSeq, Modifiers};
pub(crate) use replacementid::ReplacementEffectId;
pub(crate) use triggerid::TriggerId;

use crate::{newtype_enum::newtype_enum, player::Owner, turns::Turn};

static NEXT_BATTLEFIELD_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_GRAVEYARD_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_HAND_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_MODIFIER_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_REPLACEMENT_SEQ: AtomicUsize = AtomicUsize::new(0);
/// Starts at 1 because 0 should never be a valid stack id.
static NEXT_STACK_SEQ: AtomicUsize = AtomicUsize::new(1);

static UNIQUE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Component)]
pub(crate) struct LeftBattlefieldTurn(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Component)]
pub(crate) struct UniqueId(usize);

impl UniqueId {
    pub(crate) fn new() -> Self {
        Self(UNIQUE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
pub(crate) struct Attacking(pub(crate) Owner);

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
pub(crate)enum CastFrom {
    Hand,
    Exile,
}
}

newtype_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
    pub(crate)enum ExileReason {
        Cascade,
        Craft,
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ExiledWith(pub(crate) CardId);

impl PartialEq<CardId> for ExiledWith {
    fn eq(&self, other: &CardId) -> bool {
        self.0 == *other
    }
}

#[derive(Debug, Component)]
pub(crate) struct Active;

#[derive(Debug, Component)]
pub(crate) struct Tapped;

#[derive(Debug, Component)]
pub(crate) struct Temporary;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct Global;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct EntireBattlefield;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InLibrary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InHand(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Hash)]
pub struct InStack(usize);

impl Neg for InStack {
    type Output = i32;

    fn neg(self) -> Self::Output {
        -(self.0 as i32)
    }
}

impl InStack {
    pub(crate) fn title(self, db: &mut Database) -> String {
        if let Some(found) = db
            .query::<(Entity, &InStack)>()
            .iter(db)
            .find_map(|(card, loc)| {
                if *loc == self {
                    Some(CardId(card))
                } else {
                    None
                }
            })
        {
            return format!("Stack ({}): {}", self, found.name(db));
        }

        if let Some(found) = db
            .abilities
            .query::<(Entity, &InStack)>()
            .iter(&db.abilities)
            .find_map(|(ability, loc)| {
                if *loc == self {
                    Some(AbilityId::from(ability))
                } else {
                    None
                }
            })
        {
            return format!("Stack ({}): {}", self, found.short_text(db));
        }

        let found = db
            .triggers
            .query::<(Entity, &InStack)>()
            .iter(&db.triggers)
            .find_map(|(trigger, loc)| {
                if *loc == self {
                    Some(TriggerId::from(trigger))
                } else {
                    None
                }
            })
            .expect("Should have a valid stack target");

        found.short_text(db)
    }
}

impl From<TriggerInStack> for InStack {
    fn from(value: TriggerInStack) -> Self {
        Self(value.seq)
    }
}

impl std::fmt::Display for InStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component, Hash)]
pub(crate) struct TriggerInStack {
    pub(crate) seq: usize,
    pub(crate) source: CardId,
    pub(crate) trigger: TriggerId,
}

impl Neg for TriggerInStack {
    type Output = i32;

    fn neg(self) -> Self::Output {
        -(self.seq as i32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Default)]
pub struct OnBattlefield(usize);

impl OnBattlefield {
    pub(crate) fn new() -> Self {
        Self(NEXT_BATTLEFIELD_SEQ.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InGraveyard(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InExile;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct IsToken;

#[derive(Debug, Clone, Component, Deref, DerefMut, Default)]
pub(crate) struct Modifying(HashSet<CardId>);

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct FaceDown;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct Transformed;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct Manifested;

pub(crate) fn all_cards(db: &mut Database) -> Vec<CardId> {
    db.query::<Entity>().iter(db).map(CardId).collect()
}

pub fn cards<Location: Component + Ord>(db: &mut Database) -> Vec<CardId> {
    db.query::<(Entity, &Location)>()
        .iter(db)
        .sorted_by_key(|(_, loc)| *loc)
        .map(|(card, _)| CardId(card))
        .collect()
}

#[derive(Debug, Resource)]
pub(crate) struct NumberOfAttackers {
    pub(crate) count: usize,
    pub(crate) turn: usize,
}

pub(crate) fn number_of_attackers_this_turn(db: &Database, turn: &Turn) -> usize {
    if let Some(number) = db.get_resource::<NumberOfAttackers>() {
        if number.turn == turn.turn_count {
            number.count
        } else {
            0
        }
    } else {
        0
    }
}

#[derive(Debug, Resource)]
pub(crate) struct LifeGained {
    pub(crate) counts: HashMap<Owner, usize>,
}

pub(crate) fn update_life_gained_this_turn(db: &mut Database, player: Owner, amount: usize) {
    if let Some(mut number) = db.get_resource_mut::<LifeGained>() {
        *number.counts.entry(player).or_default() += amount
    } else {
        db.insert_resource(LifeGained {
            counts: HashMap::from([(player, amount)]),
        })
    }
}

pub(crate) fn life_gained_this_turn(db: &Database, player: Owner) -> usize {
    if let Some(number) = db.get_resource::<LifeGained>() {
        number.counts.get(&player).copied().unwrap_or_default()
    } else {
        0
    }
}

#[derive(Debug, Resource)]
pub(crate) struct TimesDescended {
    counts: HashMap<Owner, usize>,
}

pub(crate) fn descend(db: &mut Database, player: Owner) {
    if let Some(mut number) = db.get_resource_mut::<LifeGained>() {
        *number.counts.entry(player).or_default() += 1;
    } else {
        db.insert_resource(LifeGained {
            counts: HashMap::from([(player, 1)]),
        })
    }
}

pub(crate) fn times_descended_this_turn(db: &Database, player: Owner) -> usize {
    if let Some(number) = db.get_resource::<TimesDescended>() {
        number.counts.get(&player).copied().unwrap_or_default()
    } else {
        0
    }
}

#[derive(Debug, Deref, DerefMut, Default)]
pub struct CardDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub(crate) struct ModifierDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub(crate) struct TriggerDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub(crate) struct ActivatedAbilityDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub(crate) struct AurasDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub(crate) struct CountersDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub(crate) struct ReplacementDb(World);

#[derive(Debug, Deref, DerefMut)]
pub struct Database {
    #[deref]
    #[deref_mut]
    pub(crate) cards: CardDb,
    pub(crate) modifiers: ModifierDb,
    pub(crate) triggers: TriggerDb,
    pub(crate) abilities: ActivatedAbilityDb,
    pub(crate) auras: AurasDb,
    pub(crate) counters: CountersDb,
    pub(crate) replacement_effects: ReplacementDb,
}

impl Default for Database {
    fn default() -> Self {
        let mut cards = CardDb::default();
        cards.insert_resource(Events::<DeleteAbility>::default());

        Self {
            cards,
            modifiers: Default::default(),
            triggers: Default::default(),
            abilities: Default::default(),
            auras: Default::default(),
            counters: Default::default(),
            replacement_effects: Default::default(),
        }
    }
}
