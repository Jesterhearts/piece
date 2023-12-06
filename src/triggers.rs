use anyhow::anyhow;
use enumset::{EnumSet, EnumSetType};

use crate::{protogen, types::Type};

#[derive(Debug, EnumSetType)]
pub enum Location {
    Anywhere,
    Battlefield,
    Library,
}

impl TryFrom<&protogen::triggers::Location> for Location {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::triggers::Location) -> Result<Self, Self::Error> {
        value
            .location
            .as_ref()
            .ok_or_else(|| anyhow!("Expected location to have a location specified."))
            .map(Location::from)
    }
}

impl From<&protogen::triggers::location::Location> for Location {
    fn from(value: &protogen::triggers::location::Location) -> Self {
        match value {
            protogen::triggers::location::Location::Anywhere(_) => Self::Anywhere,
            protogen::triggers::location::Location::Battlefield(_) => Self::Battlefield,
            protogen::triggers::location::Location::Library(_) => Self::Library,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutIntoGraveyard {
    pub location: Location,
    pub types: EnumSet<Type>,
}

impl TryFrom<&protogen::triggers::trigger::PutIntoGraveyard> for PutIntoGraveyard {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::triggers::trigger::PutIntoGraveyard,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            location: value.from.get_or_default().try_into()?,
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    PutIntoGraveyard(PutIntoGraveyard),
}

impl TryFrom<&protogen::triggers::Trigger> for Trigger {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::triggers::Trigger) -> Result<Self, Self::Error> {
        value
            .trigger
            .as_ref()
            .ok_or_else(|| anyhow!("Expected trigger to have a trigger specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::triggers::trigger::Trigger> for Trigger {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::triggers::trigger::Trigger) -> Result<Self, Self::Error> {
        match value {
            protogen::triggers::trigger::Trigger::PutIntoGraveyard(trigger) => {
                Ok(Self::PutIntoGraveyard(trigger.try_into()?))
            }
        }
    }
}
