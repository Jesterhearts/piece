use anyhow::anyhow;
use enumset::EnumSetType;

use crate::{
    controller::Controller,
    protogen,
    types::{Subtype, Type},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpellTarget {
    pub controller: Controller,
    pub types: Vec<Type>,
    pub subtypes: Vec<Subtype>,
}

impl TryFrom<&protogen::targets::SpellTarget> for SpellTarget {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::SpellTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            controller: value
                .controller
                .controller
                .as_ref()
                .map(Controller::from)
                .unwrap_or_default(),
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, EnumSetType)]
pub enum Restriction {
    NotSelf,
    SingleTarget,
    CreaturesOnly,
}

impl TryFrom<&protogen::targets::Restriction> for Restriction {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::Restriction) -> Result<Self, Self::Error> {
        value
            .restriction
            .as_ref()
            .ok_or_else(|| anyhow!("Expected restriction to have a restriction specified"))
            .map(Restriction::from)
    }
}

impl From<&protogen::targets::restriction::Restriction> for Restriction {
    fn from(value: &protogen::targets::restriction::Restriction) -> Self {
        match value {
            protogen::targets::restriction::Restriction::NotSelf(_) => Self::NotSelf,
            protogen::targets::restriction::Restriction::SingleTarget(_) => Self::SingleTarget,
            protogen::targets::restriction::Restriction::CreaturesOnly(_) => Self::CreaturesOnly,
        }
    }
}
