use anyhow::anyhow;
use enumset::EnumSet;

use crate::{
    controller::Controller,
    cost::AbilityCost,
    effects::{ActivatedAbilityEffect, BattlefieldModifier},
    protogen,
    targets::Restriction,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Enchant {
    pub modifiers: Vec<BattlefieldModifier>,
    pub restrictions: EnumSet<Restriction>,
}

impl TryFrom<&protogen::abilities::static_ability::Enchant> for Enchant {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::static_ability::Enchant) -> Result<Self, Self::Error> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ETBAbility {
    CopyOfAnyCreature,
}

impl TryFrom<&protogen::abilities::ETBAbility> for ETBAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::ETBAbility) -> Result<Self, Self::Error> {
        value
            .ability
            .as_ref()
            .ok_or_else(|| anyhow!("Expected etb ability to have an ability specified"))
            .map(Self::from)
    }
}

impl From<&protogen::abilities::etbability::Ability> for ETBAbility {
    fn from(value: &protogen::abilities::etbability::Ability) -> Self {
        match value {
            protogen::abilities::etbability::Ability::CopyOfAnyCreature(_) => {
                Self::CopyOfAnyCreature
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StaticAbility {
    GreenCannotBeCountered { controller: Controller },
    Vigilance,
    BattlefieldModifier(BattlefieldModifier),
    Enchant(Enchant),
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
            protogen::abilities::static_ability::Ability::Enchant(enchant) => {
                Ok(Self::Enchant(enchant.try_into()?))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
