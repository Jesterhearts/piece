use std::collections::HashSet;

use anyhow::anyhow;
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};

use crate::{
    controller::ControllerRestriction,
    protogen,
    types::{Subtype, Type},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Comparison {
    LessThan(i32),
    LessThanOrEqual(i32),
}

impl TryFrom<&protogen::targets::Comparison> for Comparison {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::Comparison) -> Result<Self, Self::Error> {
        value
            .value
            .as_ref()
            .ok_or_else(|| anyhow!("Expected comparison to have a comparison specified"))
            .map(Comparison::from)
    }
}

impl From<&protogen::targets::comparison::Value> for Comparison {
    fn from(value: &protogen::targets::comparison::Value) -> Self {
        match value {
            protogen::targets::comparison::Value::LessThan(value) => Self::LessThan(value.value),
            protogen::targets::comparison::Value::LessThanOrEqual(value) => {
                Self::LessThanOrEqual(value.value)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpellTarget {
    pub controller: ControllerRestriction,
    pub types: HashSet<Type>,
    pub subtypes: HashSet<Subtype>,
}

impl TryFrom<&protogen::targets::SpellTarget> for SpellTarget {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::SpellTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            controller: value
                .controller
                .controller
                .as_ref()
                .map(ControllerRestriction::from)
                .unwrap_or_default(),
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct Restrictions(pub Vec<Restriction>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Restriction {
    NotSelf,
    Self_,
    OfType {
        types: HashSet<Type>,
        subtypes: HashSet<Subtype>,
    },
    Toughness(Comparison),
    ControllerControlsBlackOrGreen,
}

impl TryFrom<&protogen::targets::Restriction> for Restriction {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::Restriction) -> Result<Self, Self::Error> {
        value
            .restriction
            .as_ref()
            .ok_or_else(|| anyhow!("Expected restriction to have a restriction specified"))
            .and_then(Restriction::try_from)
    }
}

impl TryFrom<&protogen::targets::restriction::Restriction> for Restriction {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::restriction::Restriction) -> Result<Self, Self::Error> {
        match value {
            protogen::targets::restriction::Restriction::NotSelf(_) => Ok(Self::NotSelf),
            protogen::targets::restriction::Restriction::Self_(_) => Ok(Self::Self_),
            protogen::targets::restriction::Restriction::OfType(types) => Ok(Self::OfType {
                types: types
                    .types
                    .iter()
                    .map(Type::try_from)
                    .collect::<anyhow::Result<HashSet<_>>>()?,
                subtypes: types
                    .subtypes
                    .iter()
                    .map(Subtype::try_from)
                    .collect::<anyhow::Result<HashSet<_>>>()?,
            }),
            protogen::targets::restriction::Restriction::Toughness(toughness) => Ok(
                Self::Toughness(toughness.comparison.get_or_default().try_into()?),
            ),
            protogen::targets::restriction::Restriction::ControllerControlsBlackOrGreen(_) => {
                Ok(Self::ControllerControlsBlackOrGreen)
            }
        }
    }
}
