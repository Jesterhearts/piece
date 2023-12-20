use anyhow::anyhow;

use crate::{
    controller::ControllerRestriction, newtype_enum::newtype_enum, protogen, targets::Restriction,
};

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
pub enum Location {
    Anywhere,
    Battlefield,
    Hand,
    Library,
}
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

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
pub enum TriggerSource {
    Cast,
    PutIntoGraveyard,
    EntersTheBattlefield,
    Tapped,
}
}

#[derive(Debug, Clone, PartialEq, Eq, bevy_ecs::component::Component)]
pub struct Trigger {
    pub trigger: TriggerSource,
    pub from: Location,
    pub controller: ControllerRestriction,
    pub restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::triggers::Trigger> for Trigger {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::triggers::Trigger) -> Result<Self, Self::Error> {
        Ok(Self {
            trigger: value.source.get_or_default().try_into()?,
            from: value.from.get_or_default().try_into()?,
            controller: value.controller.get_or_default().try_into()?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl TryFrom<&protogen::triggers::TriggerSource> for TriggerSource {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::triggers::TriggerSource) -> Result<Self, Self::Error> {
        value
            .trigger
            .as_ref()
            .ok_or_else(|| anyhow!("Expected trigger to have a trigger specified"))
            .map(Self::from)
    }
}

impl From<&protogen::triggers::trigger_source::Trigger> for TriggerSource {
    fn from(value: &protogen::triggers::trigger_source::Trigger) -> Self {
        match value {
            protogen::triggers::trigger_source::Trigger::PutIntoGraveyard(_) => {
                Self::PutIntoGraveyard
            }
            protogen::triggers::trigger_source::Trigger::EntersTheBattlefield(_) => {
                Self::EntersTheBattlefield
            }
            protogen::triggers::trigger_source::Trigger::Tapped(_) => Self::Tapped,
        }
    }
}
