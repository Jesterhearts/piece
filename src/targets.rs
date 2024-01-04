use std::{collections::HashSet, str::FromStr};

use anyhow::{anyhow, Context};
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;

use crate::{
    card::{Color, Keyword},
    counters::Counter,
    player::mana_pool::ManaSource,
    protogen,
    types::{parse_subtype_list, parse_type_list, Subtype, Type},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::AsRefStr)]
#[allow(unused)]
pub(crate) enum Location {
    Battlefield,
    Graveyard,
    Exile,
    Library,
    Hand,
    Stack,
}

impl TryFrom<&protogen::targets::Location> for Location {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::Location) -> Result<Self, Self::Error> {
        value
            .location
            .as_ref()
            .ok_or_else(|| anyhow!("Expected location to have a location set"))
            .map(Self::from)
    }
}

impl From<&protogen::targets::location::Location> for Location {
    fn from(value: &protogen::targets::location::Location) -> Self {
        match value {
            protogen::targets::location::Location::OnBattlefield(_) => Self::Battlefield,
            protogen::targets::location::Location::InGraveyard(_) => Self::Graveyard,
            protogen::targets::location::Location::InLibrary(_) => Self::Library,
            protogen::targets::location::Location::InExile(_) => Self::Exile,
            protogen::targets::location::Location::InStack(_) => Self::Stack,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Comparison {
    LessThan(i32),
    LessThanOrEqual(i32),
    GreaterThan(i32),
    GreaterThanOrEqual(i32),
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

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub(crate) struct Restrictions(pub(crate) Vec<Restriction>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Dynamic {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cmc {
    Comparison(Comparison),
    Dynamic(Dynamic),
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

#[derive(Debug, Clone, Copy, strum::AsRefStr)]
pub enum ControllerRestriction {
    Self_,
    Opponent,
}

impl TryFrom<&protogen::targets::restriction::Controller> for ControllerRestriction {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::restriction::Controller) -> Result<Self, Self::Error> {
        value
            .controller
            .as_ref()
            .ok_or_else(|| anyhow!("Expected controller to have a controller set."))
            .map(Self::from)
    }
}

impl From<&protogen::targets::restriction::controller::Controller> for ControllerRestriction {
    fn from(value: &protogen::targets::restriction::controller::Controller) -> Self {
        match value {
            protogen::targets::restriction::controller::Controller::Self_(_) => Self::Self_,
            protogen::targets::restriction::controller::Controller::Opponent(_) => Self::Opponent,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Restriction {
    AttackedThisTurn,
    Attacking,
    AttackingOrBlocking,
    CastFromHand,
    Cmc(Cmc),
    Controller(ControllerRestriction),
    ControllerControlsBlackOrGreen,
    ControllerHandEmpty,
    Descend(usize),
    DescendedThisTurn,
    DuringControllersTurn,
    EnteredTheBattlefieldThisTurn {
        count: usize,
        restrictions: Vec<Restriction>,
    },
    InGraveyard,
    InLocation {
        locations: Vec<Location>,
    },
    JustCast,
    LifeGainedThisTurn(usize),
    ManaSpentFromSource(ManaSource),
    NotChosen,
    NotKeywords(IndexSet<Keyword>),
    NotOfType {
        types: IndexSet<Type>,
        subtypes: IndexSet<Subtype>,
    },
    NotSelf,
    NumberOfCountersOnThis {
        comparison: Comparison,
        counter: Counter,
    },
    OfColor(HashSet<Color>),
    OfType {
        types: IndexSet<Type>,
        subtypes: IndexSet<Subtype>,
    },
    OnBattlefield,
    Power(Comparison),
    Self_,
    SourceCast,
    Tapped,
    Threshold,
    Toughness(Comparison),
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
            protogen::targets::restriction::Restriction::AttackedThisTurn(_) => {
                Ok(Self::AttackedThisTurn)
            }
            protogen::targets::restriction::Restriction::Attacking(_) => Ok(Self::Attacking),
            protogen::targets::restriction::Restriction::AttackingOrBlocking(_) => {
                Ok(Self::AttackingOrBlocking)
            }
            protogen::targets::restriction::Restriction::CastFromHand(_) => Ok(Self::CastFromHand),
            protogen::targets::restriction::Restriction::Cmc(cmc) => Ok(Self::Cmc(cmc.try_into()?)),
            protogen::targets::restriction::Restriction::Controller(controller) => {
                Ok(Self::Controller(controller.try_into()?))
            }
            protogen::targets::restriction::Restriction::ControllerControlsBlackOrGreen(_) => {
                Ok(Self::ControllerControlsBlackOrGreen)
            }
            protogen::targets::restriction::Restriction::ControllerHandEmpty(_) => {
                Ok(Self::ControllerHandEmpty)
            }
            protogen::targets::restriction::Restriction::Descend(value) => {
                Ok(Self::Descend(usize::try_from(value.count)?))
            }
            protogen::targets::restriction::Restriction::DescendedThisTurn(_) => {
                Ok(Self::DescendedThisTurn)
            }
            protogen::targets::restriction::Restriction::DuringControllersTurn(_) => {
                Ok(Self::DuringControllersTurn)
            }
            protogen::targets::restriction::Restriction::EnteredBattlefieldThisTurn(value) => {
                Ok(Self::EnteredTheBattlefieldThisTurn {
                    count: usize::try_from(value.count)?,
                    restrictions: value
                        .restrictions
                        .iter()
                        .map(Restriction::try_from)
                        .collect::<anyhow::Result<_>>()?,
                })
            }
            protogen::targets::restriction::Restriction::InGraveyard(_) => Ok(Self::InGraveyard),
            protogen::targets::restriction::Restriction::JustCast(_) => Ok(Self::JustCast),
            protogen::targets::restriction::Restriction::LifeGainedThisTurn(value) => {
                Ok(Self::LifeGainedThisTurn(usize::try_from(value.count)?))
            }
            protogen::targets::restriction::Restriction::Location(value) => Ok(Self::InLocation {
                locations: value
                    .locations
                    .iter()
                    .map(Location::try_from)
                    .collect::<anyhow::Result<_>>()?,
            }),
            protogen::targets::restriction::Restriction::ManaSpentFromSource(spent) => Ok(
                Self::ManaSpentFromSource(spent.source.get_or_default().try_into()?),
            ),
            protogen::targets::restriction::Restriction::NotChosen(_) => Ok(Self::NotChosen),
            protogen::targets::restriction::Restriction::NotKeywords(not) => Ok(Self::NotKeywords(
                not.keywords
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| Keyword::from_str(s.trim()).with_context(|| anyhow!("Parsing {}", s)))
                    .collect::<anyhow::Result<_>>()?,
            )),
            protogen::targets::restriction::Restriction::NotOfType(not) => {
                let types = parse_type_list(&not.types)?;
                let subtypes = parse_subtype_list(&not.subtypes)?;
                Ok(Self::NotOfType { types, subtypes })
            }
            protogen::targets::restriction::Restriction::NotSelf(_) => Ok(Self::NotSelf),
            protogen::targets::restriction::Restriction::NumberOfCountersOnThis(value) => {
                Ok(Self::NumberOfCountersOnThis {
                    comparison: value.comparison.get_or_default().try_into()?,
                    counter: (&value.counter).try_into()?,
                })
            }
            protogen::targets::restriction::Restriction::OfColor(colors) => Ok(Self::OfColor(
                colors
                    .colors
                    .iter()
                    .map(Color::try_from)
                    .collect::<anyhow::Result<_>>()?,
            )),
            protogen::targets::restriction::Restriction::OfType(of_type) => {
                let types = parse_type_list(&of_type.types)?;
                let subtypes = parse_subtype_list(&of_type.subtypes)?;

                Ok(Self::OfType { types, subtypes })
            }
            protogen::targets::restriction::Restriction::OnBattlefield(_) => {
                Ok(Self::OnBattlefield)
            }
            protogen::targets::restriction::Restriction::Power(power) => {
                Ok(Self::Power(power.comparison.get_or_default().try_into()?))
            }
            protogen::targets::restriction::Restriction::Self_(_) => Ok(Self::Self_),
            protogen::targets::restriction::Restriction::SourceCast(_) => Ok(Self::SourceCast),
            protogen::targets::restriction::Restriction::Tapped(_) => Ok(Self::Tapped),
            protogen::targets::restriction::Restriction::Threshold(_) => Ok(Self::Threshold),
            protogen::targets::restriction::Restriction::Toughness(toughness) => Ok(
                Self::Toughness(toughness.comparison.get_or_default().try_into()?),
            ),
        }
    }
}
