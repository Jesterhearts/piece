use anyhow::anyhow;

use crate::{
    controller::Controller,
    cost::AbilityCost,
    effects::{ActivatedAbilityEffect, BattlefieldModifier},
    protogen,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ETBAbility {
    CopyOfAnyCreature,
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
                .map(|effect| {
                    effect
                        .effect
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected effect to have an effect specified"))
                        .and_then(ActivatedAbilityEffect::try_from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}
