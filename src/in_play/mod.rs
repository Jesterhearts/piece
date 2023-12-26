mod abilityid;
mod auraid;
mod cardid;
mod counterid;
mod modifierid;
mod replacementid;
mod triggerid;

use std::{
    collections::HashSet,
    ops::Neg,
    sync::atomic::{AtomicUsize, Ordering},
};

use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, Events},
    world::World,
};
use derive_more::{Deref, DerefMut};
use itertools::Itertools;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Event)]
pub struct DeleteAbility {
    pub ability: AbilityId,
}

pub use abilityid::AbilityId;
pub use auraid::AuraId;
pub use cardid::{target_from_location, CardId, Cloning};
pub use counterid::CounterId;
pub use modifierid::{ModifierId, ModifierSeq, Modifiers};
pub use replacementid::ReplacementEffectId;
pub use triggerid::TriggerId;

use crate::{newtype_enum::newtype_enum, player::Owner};

static NEXT_BATTLEFIELD_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_GRAVEYARD_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_HAND_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_MODIFIER_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_REPLACEMENT_SEQ: AtomicUsize = AtomicUsize::new(0);
/// Starts at 1 because 0 should never be a valid stack id.
static NEXT_STACK_SEQ: AtomicUsize = AtomicUsize::new(1);

static UNIQUE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Component)]
pub struct LeftBattlefieldTurn(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Component)]
pub struct UniqueId(usize);

impl UniqueId {
    pub fn new() -> Self {
        Self(UNIQUE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
pub struct Attacking(pub Owner);

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
pub enum CastFrom {
    Hand,
    Exile,
}
}

newtype_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
    pub enum ExileReason {
        Cascade,
        Craft,
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct ExiledWith(pub CardId);

impl PartialEq<CardId> for ExiledWith {
    fn eq(&self, other: &CardId) -> bool {
        self.0 == *other
    }
}

#[derive(Debug, Component)]
pub struct Active;

#[derive(Debug, Component)]
pub struct Tapped;

#[derive(Debug, Component)]
pub struct Temporary;

#[derive(Debug, Clone, Copy, Component)]
pub struct Global;

#[derive(Debug, Clone, Copy, Component)]
pub struct EntireBattlefield;

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
    pub fn title(self, db: &mut Database) -> String {
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
pub struct TriggerInStack {
    pub seq: usize,
    pub source: CardId,
    pub trigger: TriggerId,
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
    pub fn new() -> Self {
        Self(NEXT_BATTLEFIELD_SEQ.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InGraveyard(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InExile;

#[derive(Debug, Clone, Copy, Component)]
pub struct IsToken;

#[derive(Debug, Clone, Component, Deref, DerefMut, Default)]
pub struct Modifying(HashSet<CardId>);

#[derive(Debug, Clone, Copy, Component)]
pub struct FaceDown;

#[derive(Debug, Clone, Copy, Component)]
pub struct Transformed;

#[derive(Debug, Clone, Copy, Component)]
pub struct Manifested;

pub fn all_cards(db: &mut Database) -> Vec<CardId> {
    db.query::<Entity>().iter(db).map(CardId).collect()
}

pub fn cards<Location: Component + Ord>(db: &mut Database) -> Vec<CardId> {
    db.query::<(Entity, &Location)>()
        .iter(db)
        .sorted_by_key(|(_, loc)| *loc)
        .map(|(card, _)| CardId(card))
        .collect()
}

#[derive(Debug, Deref, DerefMut, Default)]
pub struct CardDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct ModifierDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct TriggerDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct ActivatedAbilityDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct StaticAbilityDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct AurasDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct CountersDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct ReplacementDb(World);

#[derive(Debug, Deref, DerefMut)]
pub struct Database {
    #[deref]
    #[deref_mut]
    pub cards: CardDb,
    pub modifiers: ModifierDb,
    pub triggers: TriggerDb,
    pub abilities: ActivatedAbilityDb,
    pub static_abilities: StaticAbilityDb,
    pub auras: AurasDb,
    pub counters: CountersDb,
    pub replacement_effects: ReplacementDb,
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
            static_abilities: Default::default(),
            auras: Default::default(),
            counters: Default::default(),
            replacement_effects: Default::default(),
        }
    }
}
