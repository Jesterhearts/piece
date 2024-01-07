use anyhow::anyhow;

use crate::{protogen, targets::Restriction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Location {
    Anywhere,
    Battlefield,
    Hand,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TriggerSource {
    Attacks,
    Cast,
    EndStep,
    EntersTheBattlefield,
    ExiledDuringCraft,
    OneOrMoreTapped,
    PreCombatMainPhase,
    PutIntoGraveyard,
    StartOfCombat,
    Tapped,
}

#[derive(Debug, Clone)]
pub(crate) struct Trigger {
    pub(crate) trigger: TriggerSource,
    pub(crate) from: Location,
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::triggers::Trigger> for Trigger {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::triggers::Trigger) -> Result<Self, Self::Error> {
        Ok(Self {
            trigger: value.source.get_or_default().try_into()?,
            from: value.from.get_or_default().try_into()?,
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
            protogen::triggers::trigger_source::Trigger::Attacks(_) => Self::Attacks,
            protogen::triggers::trigger_source::Trigger::Cast(_) => Self::Cast,
            protogen::triggers::trigger_source::Trigger::EndStep(_) => Self::EndStep,
            protogen::triggers::trigger_source::Trigger::EntersTheBattlefield(_) => {
                Self::EntersTheBattlefield
            }
            protogen::triggers::trigger_source::Trigger::ExiledDuringCraft(_) => {
                Self::ExiledDuringCraft
            }
            protogen::triggers::trigger_source::Trigger::OneOrMoreTapped(_) => {
                Self::OneOrMoreTapped
            }
            protogen::triggers::trigger_source::Trigger::PreCombatMainPhase(_) => {
                Self::PreCombatMainPhase
            }
            protogen::triggers::trigger_source::Trigger::PutIntoGraveyard(_) => {
                Self::PutIntoGraveyard
            }
            protogen::triggers::trigger_source::Trigger::StartOfCombat(_) => Self::StartOfCombat,
            protogen::triggers::trigger_source::Trigger::Tapped(_) => Self::Tapped,
        }
    }
}
