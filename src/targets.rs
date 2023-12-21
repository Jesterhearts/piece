use std::{
    collections::HashSet,
    fmt::{Display, Write},
};

use anyhow::anyhow;
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    card::Color,
    controller::ControllerRestriction,
    protogen,
    types::{Subtype, Type},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Comparison {
    LessThan(i32),
    LessThanOrEqual(i32),
    GreaterThan(i32),
    GreaterThanOrEqual(i32),
}

impl Display for Comparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Comparison::LessThan(i) => f.write_fmt(format_args!("less than {}", i)),
            Comparison::LessThanOrEqual(i) => {
                f.write_fmt(format_args!("less than or equal to {}", i))
            }
            Comparison::GreaterThan(i) => f.write_fmt(format_args!("greater than {}", i)),
            Comparison::GreaterThanOrEqual(i) => {
                f.write_fmt(format_args!("greater than or equal to {}", i))
            }
        }
    }
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
            protogen::targets::comparison::Value::GreaterThan(value) => {
                Self::GreaterThan(value.value)
            }
            protogen::targets::comparison::Value::GreaterThanOrEqual(value) => {
                Self::GreaterThanOrEqual(value.value)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpellTarget {
    pub controller: ControllerRestriction,
    pub types: IndexSet<Type>,
    pub subtypes: IndexSet<Subtype>,
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
                .collect::<anyhow::Result<_>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct Restrictions(pub Vec<Restriction>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dynamic {
    X,
}

impl TryFrom<&protogen::targets::Dynamic> for Dynamic {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::Dynamic) -> Result<Self, Self::Error> {
        value
            .dynamic
            .as_ref()
            .ok_or_else(|| anyhow!("Expected dynamic to have a value set"))
            .map(Self::from)
    }
}

impl From<&protogen::targets::dynamic::Dynamic> for Dynamic {
    fn from(value: &protogen::targets::dynamic::Dynamic) -> Self {
        match value {
            protogen::targets::dynamic::Dynamic::X(_) => Self::X,
        }
    }
}

impl std::fmt::Display for Dynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dynamic::X => f.write_char('X'),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmc {
    Comparison(Comparison),
    Dynamic(Dynamic),
}

impl std::fmt::Display for Cmc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmc::Comparison(comparison) => comparison.fmt(f),
            Cmc::Dynamic(dy) => dy.fmt(f),
        }
    }
}

impl TryFrom<&protogen::targets::restriction::Cmc> for Cmc {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::restriction::Cmc) -> Result<Self, Self::Error> {
        value
            .cmc
            .as_ref()
            .ok_or_else(|| anyhow!("Expected cmc to have a cmc set"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::targets::restriction::cmc::Cmc> for Cmc {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::restriction::cmc::Cmc) -> Result<Self, Self::Error> {
        match value {
            protogen::targets::restriction::cmc::Cmc::Dynamic(dy) => {
                Ok(Self::Dynamic(dy.try_into()?))
            }
            protogen::targets::restriction::cmc::Cmc::Comparison(c) => {
                Ok(Self::Comparison(c.try_into()?))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Restriction {
    AttackingOrBlocking,
    NotSelf,
    Self_,
    OfColor(HashSet<Color>),
    OfType {
        types: IndexSet<Type>,
        subtypes: IndexSet<Subtype>,
    },
    NotOfType {
        types: IndexSet<Type>,
        subtypes: IndexSet<Subtype>,
    },
    CastFromHand,
    Cmc(Cmc),
    Toughness(Comparison),
    ControllerControlsBlackOrGreen,
    ControllerHandEmpty,
    InGraveyard,
    OnBattlefield,
}

impl Restriction {
    pub fn text(&self) -> String {
        match self {
            Restriction::AttackingOrBlocking => "attacking or blocking".to_string(),
            Restriction::NotSelf => "other permanent".to_string(),
            Restriction::Self_ => "self".to_string(),
            Restriction::OfType { types, subtypes } => {
                if !types.is_empty() && !subtypes.is_empty() {
                    format!(
                        "a {} - {}",
                        types.iter().map(|ty| ty.as_ref()).join(" or "),
                        subtypes.iter().map(|ty| ty.as_ref()).join(" or ")
                    )
                } else if !types.is_empty() {
                    format!("a {}", types.iter().map(|ty| ty.as_ref()).join(" or "))
                } else {
                    format!("a {}", subtypes.iter().map(|ty| ty.as_ref()).join(" or "))
                }
            }
            Restriction::NotOfType { types, subtypes } => {
                if !types.is_empty() && !subtypes.is_empty() {
                    format!(
                        "not a {} - {}",
                        types.iter().map(|ty| ty.as_ref()).join(" or "),
                        subtypes.iter().map(|ty| ty.as_ref()).join(" or ")
                    )
                } else if !types.is_empty() {
                    format!("not a {}", types.iter().map(|ty| ty.as_ref()).join(" or "))
                } else {
                    format!(
                        "not a {}",
                        subtypes.iter().map(|ty| ty.as_ref()).join(" or ")
                    )
                }
            }
            Restriction::Toughness(toughness) => format!("toughness {}", toughness),
            Restriction::ControllerControlsBlackOrGreen => {
                "controller controls black or green".to_string()
            }
            Restriction::ControllerHandEmpty => "controller hand empty".to_string(),
            Restriction::OfColor(colors) => {
                format!("one of {}", colors.iter().map(|c| c.as_ref()).join(", "))
            }
            Restriction::Cmc(cmc) => format!("cmc {}", cmc),
            Restriction::CastFromHand => "cast from your hand".to_string(),
            Restriction::InGraveyard => "in a graveyard".to_string(),
            Restriction::OnBattlefield => "on the battlefield".to_string(),
        }
    }
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
            protogen::targets::restriction::Restriction::AttackingOrBlocking(_) => {
                Ok(Self::AttackingOrBlocking)
            }
            protogen::targets::restriction::Restriction::NotSelf(_) => Ok(Self::NotSelf),
            protogen::targets::restriction::Restriction::Self_(_) => Ok(Self::Self_),
            protogen::targets::restriction::Restriction::OfType(types) => Ok(Self::OfType {
                types: types
                    .types
                    .iter()
                    .map(Type::try_from)
                    .collect::<anyhow::Result<_>>()?,
                subtypes: types
                    .subtypes
                    .iter()
                    .map(Subtype::try_from)
                    .collect::<anyhow::Result<_>>()?,
            }),
            protogen::targets::restriction::Restriction::Toughness(toughness) => Ok(
                Self::Toughness(toughness.comparison.get_or_default().try_into()?),
            ),
            protogen::targets::restriction::Restriction::ControllerControlsBlackOrGreen(_) => {
                Ok(Self::ControllerControlsBlackOrGreen)
            }
            protogen::targets::restriction::Restriction::ControllerHandEmpty(_) => {
                Ok(Self::ControllerHandEmpty)
            }
            protogen::targets::restriction::Restriction::OfColor(colors) => Ok(Self::OfColor(
                colors
                    .colors
                    .iter()
                    .map(Color::try_from)
                    .collect::<anyhow::Result<_>>()?,
            )),
            protogen::targets::restriction::Restriction::Cmc(cmc) => Ok(Self::Cmc(cmc.try_into()?)),
            protogen::targets::restriction::Restriction::CastFromHand(_) => Ok(Self::CastFromHand),
            protogen::targets::restriction::Restriction::NotOfType(not) => Ok(Self::NotOfType {
                types: not
                    .types
                    .iter()
                    .map(Type::try_from)
                    .collect::<anyhow::Result<_>>()?,
                subtypes: not
                    .subtypes
                    .iter()
                    .map(Subtype::try_from)
                    .collect::<anyhow::Result<_>>()?,
            }),
            protogen::targets::restriction::Restriction::InGraveyard(_) => Ok(Self::InGraveyard),
            protogen::targets::restriction::Restriction::OnBattlefield(_) => {
                Ok(Self::OnBattlefield)
            }
        }
    }
}
