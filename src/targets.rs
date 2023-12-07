use anyhow::anyhow;
use enumset::EnumSet;

use crate::{
    controller::Controller,
    protogen,
    types::{Subtype, Type},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpellTarget {
    pub controller: Controller,
    pub types: EnumSet<Type>,
    pub subtypes: EnumSet<Subtype>,
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
                .collect::<anyhow::Result<EnumSet<_>>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Restriction {
    NotSelf,
    SingleTarget,
    Self_,
    OfType {
        types: EnumSet<Type>,
        subtypes: EnumSet<Subtype>,
    },
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
            protogen::targets::restriction::Restriction::SingleTarget(_) => Ok(Self::SingleTarget),
            protogen::targets::restriction::Restriction::Self_(_) => Ok(Self::Self_),
            protogen::targets::restriction::Restriction::OfType(types) => Ok(Self::OfType {
                types: types
                    .types
                    .iter()
                    .map(Type::try_from)
                    .collect::<anyhow::Result<EnumSet<_>>>()?,
                subtypes: types
                    .subtypes
                    .iter()
                    .map(Subtype::try_from)
                    .collect::<anyhow::Result<EnumSet<_>>>()?,
            }),
        }
    }
}
