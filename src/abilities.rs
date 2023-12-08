use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::{
    controller::Controller,
    cost::AbilityCost,
    effects::{
        ActivatedAbilityEffect, BattlefieldModifier, Mill, ReturnFromGraveyardToBattlefield,
        ReturnFromGraveyardToLibrary, TriggeredEffect, TutorLibrary,
    },
    protogen,
    targets::Restriction,
    triggers::Trigger,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enchant {
    pub modifiers: Vec<BattlefieldModifier>,
    pub restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::abilities::Enchant> for Enchant {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::Enchant) -> Result<Self, Self::Error> {
        Ok(Self {
            modifiers: value
                .modifiers
                .iter()
                .map(BattlefieldModifier::try_from)
                .collect::<anyhow::Result<_>>()?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ETBAbility {
    CopyOfAnyCreature,
    Mill(Mill),
    ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary),
    ReturnFromGraveyardToBattlefield(ReturnFromGraveyardToBattlefield),
    TutorLibrary(TutorLibrary),
}

impl TryFrom<&protogen::abilities::ETBAbility> for ETBAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::ETBAbility) -> Result<Self, Self::Error> {
        value
            .ability
            .as_ref()
            .ok_or_else(|| anyhow!("Expected etb ability to have an ability specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::abilities::etbability::Ability> for ETBAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::etbability::Ability) -> Result<Self, Self::Error> {
        match value {
            protogen::abilities::etbability::Ability::CopyOfAnyCreature(_) => {
                Ok(Self::CopyOfAnyCreature)
            }
            protogen::abilities::etbability::Ability::Mill(mill) => {
                Ok(Self::Mill(mill.try_into()?))
            }
            protogen::abilities::etbability::Ability::ReturnFromGraveyardToLibrary(ret) => {
                Ok(Self::ReturnFromGraveyardToLibrary(ret.try_into()?))
            }
            protogen::abilities::etbability::Ability::ReturnFromGraveyardToBattlefield(ret) => {
                Ok(Self::ReturnFromGraveyardToBattlefield(ret.try_into()?))
            }
            protogen::abilities::etbability::Ability::TutorLibrary(tutor) => {
                Ok(Self::TutorLibrary(tutor.try_into()?))
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum StaticAbility {
    GreenCannotBeCountered { controller: Controller },
    Vigilance,
    Flash,
    BattlefieldModifier(BattlefieldModifier),
}

impl TryFrom<&protogen::abilities::StaticAbility> for StaticAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::StaticAbility) -> Result<Self, Self::Error> {
        value
            .ability
            .as_ref()
            .ok_or_else(|| anyhow!("Expected ability to have an ability specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::abilities::static_ability::Ability> for StaticAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::static_ability::Ability) -> Result<Self, Self::Error> {
        match value {
            protogen::abilities::static_ability::Ability::GreenCannotBeCountered(ability) => {
                Ok(Self::GreenCannotBeCountered {
                    controller: ability
                        .controller
                        .controller
                        .as_ref()
                        .map(Controller::from)
                        .unwrap_or_default(),
                })
            }
            protogen::abilities::static_ability::Ability::BattlefieldModifier(modifier) => {
                Ok(Self::BattlefieldModifier(modifier.try_into()?))
            }
            protogen::abilities::static_ability::Ability::Vigilance(_) => Ok(Self::Vigilance),
            protogen::abilities::static_ability::Ability::Flash(_) => Ok(Self::Flash),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ActivatedAbility {
    pub cost: AbilityCost,
    pub effects: Vec<ActivatedAbilityEffect>,
}

impl TryFrom<&protogen::abilities::ActivatedAbility> for ActivatedAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::ActivatedAbility) -> Result<Self, Self::Error> {
        Ok(Self {
            cost: value
                .cost
                .as_ref()
                .ok_or_else(|| anyhow!("Expected ability to have a cost"))
                .and_then(AbilityCost::try_from)?,
            effects: value
                .effects
                .iter()
                .map(ActivatedAbilityEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggeredAbility {
    pub trigger: Trigger,
    pub effects: Vec<TriggeredEffect>,
}

impl TryFrom<&protogen::abilities::TriggeredAbility> for TriggeredAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::TriggeredAbility) -> Result<Self, Self::Error> {
        Ok(Self {
            trigger: value.trigger.get_or_default().try_into()?,
            effects: value
                .effects
                .iter()
                .map(TriggeredEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}
