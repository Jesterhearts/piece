use std::collections::HashSet;

use anyhow::anyhow;
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};

use crate::protogen;

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub struct Types(pub HashSet<Type>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct AddTypes(pub HashSet<Type>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    BasicLand,
    Land,
    Instant,
    Sorcery,
    Creature,
    Artifact,
    Enchantment,
    Battle,
    Legendary,
}

impl TryFrom<&protogen::types::Type> for Type {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::types::Type) -> Result<Self, Self::Error> {
        value
            .ty
            .as_ref()
            .ok_or_else(|| anyhow!("Expected type to have a type set"))
            .map(Self::from)
    }
}

impl From<&protogen::types::type_::Ty> for Type {
    fn from(value: &protogen::types::type_::Ty) -> Self {
        match value {
            protogen::types::type_::Ty::BasicLand(_) => Self::BasicLand,
            protogen::types::type_::Ty::Land(_) => Self::Land,
            protogen::types::type_::Ty::Instant(_) => Self::Instant,
            protogen::types::type_::Ty::Sorcery(_) => Self::Sorcery,
            protogen::types::type_::Ty::Creature(_) => Self::Creature,
            protogen::types::type_::Ty::Artifact(_) => Self::Artifact,
            protogen::types::type_::Ty::Enchantment(_) => Self::Enchantment,
            protogen::types::type_::Ty::Battle(_) => Self::Battle,
            protogen::types::type_::Ty::Legendary(_) => Self::Legendary,
        }
    }
}

impl Type {
    pub fn is_permanent(&self) -> bool {
        !matches!(self, Type::Instant | Type::Sorcery)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct Subtypes(pub HashSet<Subtype>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct AddSubtypes(pub HashSet<Subtype>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct RemoveAllSubtypes;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subtype {
    Angel,
    Aura,
    Bear,
    Dinosaur,
    Dryad,
    Elemental,
    Elf,
    Equipment,
    Forest,
    Golem,
    Human,
    Island,
    Mountain,
    Nymph,
    Plains,
    Praetor,
    Shade,
    Shaman,
    Shapeshifter,
    Soldier,
    Swamp,
}

impl TryFrom<&protogen::types::Subtype> for Subtype {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::types::Subtype) -> Result<Self, Self::Error> {
        value
            .subtype
            .as_ref()
            .ok_or_else(|| anyhow!("Expected subtype to have a subtype specified"))
            .map(Subtype::from)
    }
}

impl From<&protogen::types::subtype::Subtype> for Subtype {
    fn from(value: &protogen::types::subtype::Subtype) -> Self {
        match value {
            protogen::types::subtype::Subtype::Angel(_) => Self::Angel,
            protogen::types::subtype::Subtype::Aura(_) => Self::Aura,
            protogen::types::subtype::Subtype::Bear(_) => Self::Bear,
            protogen::types::subtype::Subtype::Dinosaur(_) => Self::Dinosaur,
            protogen::types::subtype::Subtype::Dryad(_) => Self::Dryad,
            protogen::types::subtype::Subtype::Elemental(_) => Self::Elemental,
            protogen::types::subtype::Subtype::Elf(_) => Self::Elf,
            protogen::types::subtype::Subtype::Equipment(_) => Self::Equipment,
            protogen::types::subtype::Subtype::Forest(_) => Self::Forest,
            protogen::types::subtype::Subtype::Golem(_) => Self::Golem,
            protogen::types::subtype::Subtype::Human(_) => Self::Human,
            protogen::types::subtype::Subtype::Island(_) => Self::Island,
            protogen::types::subtype::Subtype::Mountain(_) => Self::Mountain,
            protogen::types::subtype::Subtype::Nymph(_) => Self::Nymph,
            protogen::types::subtype::Subtype::Plains(_) => Self::Plains,
            protogen::types::subtype::Subtype::Praetor(_) => Self::Praetor,
            protogen::types::subtype::Subtype::Shade(_) => Self::Shade,
            protogen::types::subtype::Subtype::Shaman(_) => Self::Shaman,
            protogen::types::subtype::Subtype::Shapeshifter(_) => Self::Shapeshifter,
            protogen::types::subtype::Subtype::Soldier(_) => Self::Soldier,
            protogen::types::subtype::Subtype::Swamp(_) => Self::Swamp,
        }
    }
}
