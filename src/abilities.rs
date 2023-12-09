use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::{
    controller::Controller,
    cost::AbilityCost,
    effects::{
        BattlefieldModifier, GainMana, Mill, ModifyBattlefield, ReturnFromGraveyardToBattlefield,
        ReturnFromGraveyardToLibrary, TriggeredEffect, TutorLibrary,
    },
    protogen,
    targets::{Restriction, SpellTarget},
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
    BattlefieldModifier(BattlefieldModifier),
    ExtraLandsPerTurn(usize),
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
            protogen::abilities::static_ability::Ability::ExtraLandsPerTurn(extra_lands) => {
                Ok(Self::ExtraLandsPerTurn(usize::try_from(extra_lands.count)?))
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum ActivatedAbilityEffect {
    CounterSpell { target: SpellTarget },
    GainMana { mana: GainMana },
    BattlefieldModifier(BattlefieldModifier),
    ControllerDrawCards(usize),
    Equip(Vec<ModifyBattlefield>),
}

impl ActivatedAbilityEffect {
    pub fn wants_targets(&self) -> usize {
        match self {
            ActivatedAbilityEffect::CounterSpell { .. } => 1,
            ActivatedAbilityEffect::GainMana { .. } => 0,
            ActivatedAbilityEffect::BattlefieldModifier(_) => 0,
            ActivatedAbilityEffect::ControllerDrawCards(_) => 0,
            ActivatedAbilityEffect::Equip(_) => 1,
        }
    }
}

impl TryFrom<&protogen::effects::ActivatedAbilityEffect> for ActivatedAbilityEffect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ActivatedAbilityEffect) -> Result<Self, Self::Error> {
        value
            .effect
            .as_ref()
            .ok_or_else(|| anyhow!("Expected effect to have an effect specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::effects::activated_ability_effect::Effect> for ActivatedAbilityEffect {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::activated_ability_effect::Effect,
    ) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::activated_ability_effect::Effect::CounterSpell(counter) => {
                Ok(Self::CounterSpell {
                    target: counter.target.as_ref().unwrap_or_default().try_into()?,
                })
            }
            protogen::effects::activated_ability_effect::Effect::GainMana(gain) => {
                Ok(Self::GainMana {
                    mana: GainMana::try_from(gain)?,
                })
            }
            protogen::effects::activated_ability_effect::Effect::BattlefieldModifier(modifier) => {
                Ok(Self::BattlefieldModifier(modifier.try_into()?))
            }
            protogen::effects::activated_ability_effect::Effect::ControllerDrawCards(draw) => {
                Ok(Self::ControllerDrawCards(usize::try_from(draw.count)?))
            }
            protogen::effects::activated_ability_effect::Effect::Equip(modifier) => {
                Ok(Self::Equip(
                    modifier
                        .modifiers
                        .iter()
                        .map(ModifyBattlefield::try_from)
                        .collect::<anyhow::Result<Vec<_>>>()?,
                ))
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ActivatedAbility {
    pub cost: AbilityCost,
    pub effects: Vec<ActivatedAbilityEffect>,
    pub apply_to_self: bool,
}

impl TryFrom<&protogen::effects::ActivatedAbility> for ActivatedAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ActivatedAbility) -> Result<Self, Self::Error> {
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
            apply_to_self: value.apply_to_self,
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
