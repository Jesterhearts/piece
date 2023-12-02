use anyhow::anyhow;

use crate::{controller::Controller, mana::Mana, protogen, targets::SpellTarget, types::Subtype};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum EffectDuration {
    UntilEndOfTurn,
}

impl From<&protogen::effects::duration::Duration> for EffectDuration {
    fn from(value: &protogen::effects::duration::Duration) -> Self {
        match value {
            protogen::effects::duration::Duration::UntilEndOfTurn(_) => Self::UntilEndOfTurn,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum GainMana {
    Specific { gains: Vec<Mana> },
    Choice { choices: Vec<Vec<Mana>> },
}

impl TryFrom<&protogen::effects::gain_mana::Gain> for GainMana {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::gain_mana::Gain) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::gain_mana::Gain::Specific(specific) => Ok(Self::Specific {
                gains: specific
                    .gains
                    .iter()
                    .map(|mana| {
                        mana.mana
                            .as_ref()
                            .ok_or_else(|| anyhow!("Expected mana to have a mana field specified"))
                            .and_then(Mana::try_from)
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?,
            }),
            protogen::effects::gain_mana::Gain::Choice(choice) => Ok(Self::Choice {
                choices: choice
                    .choices
                    .iter()
                    .map(|choice| {
                        choice
                            .gains
                            .iter()
                            .map(|mana| {
                                mana.mana
                                    .as_ref()
                                    .ok_or_else(|| {
                                        anyhow!("Expected mana to have a mana field specified")
                                    })
                                    .and_then(Mana::try_from)
                            })
                            .collect::<anyhow::Result<Vec<_>>>()
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?,
            }),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModifyBasePowerToughness {
    pub targets: Vec<Subtype>,
    pub power: usize,
    pub toughness: usize,
}

impl TryFrom<&protogen::effects::ModifyBasePowerToughness> for ModifyBasePowerToughness {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ModifyBasePowerToughness) -> Result<Self, Self::Error> {
        Ok(Self {
            targets: value
                .targets
                .iter()
                .map(|target| {
                    target
                        .subtype
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected subtype to have a subtype specified"))
                        .map(Subtype::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            power: usize::try_from(value.power)?,
            toughness: usize::try_from(value.toughness)?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ModifyCreatureTypes {
    pub targets: Vec<Subtype>,
    pub types: Vec<Subtype>,
}

impl TryFrom<&protogen::effects::ModifyCreatureTypes> for ModifyCreatureTypes {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ModifyCreatureTypes) -> Result<Self, Self::Error> {
        Ok(Self {
            targets: value
                .targets
                .iter()
                .map(|target| {
                    target
                        .subtype
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected subtype to have a subtype specified"))
                        .map(Subtype::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            types: value
                .types
                .iter()
                .map(|ty| {
                    ty.subtype
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected subtype to have a subtype specified"))
                        .map(Subtype::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AddPowerToughness {
    pub power: usize,
    pub toughness: usize,
}

impl TryFrom<&protogen::effects::AddPowerToughness> for AddPowerToughness {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::AddPowerToughness) -> Result<Self, Self::Error> {
        Ok(Self {
            power: usize::try_from(value.power)?,
            toughness: usize::try_from(value.toughness)?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ModifyBattlefield {
    ModifyBasePowerToughness(ModifyBasePowerToughness),
    ModifyCreatureTypes(ModifyCreatureTypes),
    AddPowerToughness(AddPowerToughness),
}

impl TryFrom<&protogen::effects::modify_battlefield::Modifier> for ModifyBattlefield {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::modify_battlefield::Modifier,
    ) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::modify_battlefield::Modifier::ModifyBasePowerToughness(modifier) => {
                Ok(Self::ModifyBasePowerToughness(modifier.try_into()?))
            }
            protogen::effects::modify_battlefield::Modifier::ModifyCreatureTypes(modifier) => {
                Ok(Self::ModifyCreatureTypes(modifier.try_into()?))
            }
            protogen::effects::modify_battlefield::Modifier::AddPowerToughness(modifier) => {
                Ok(Self::AddPowerToughness(modifier.try_into()?))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct BattlefieldModifier {
    pub modifier: ModifyBattlefield,
    pub controller: Controller,
    pub duration: EffectDuration,
}

impl TryFrom<&protogen::effects::BattlefieldModifier> for BattlefieldModifier {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::BattlefieldModifier) -> Result<Self, Self::Error> {
        Ok(Self {
            modifier: value
                .modifier
                .modifier
                .as_ref()
                .ok_or_else(|| anyhow!("Expected battlefield modifier to have a modifier set"))?
                .try_into()?,
            controller: value
                .controller
                .controller
                .as_ref()
                .ok_or_else(|| anyhow!("Expected battlefield modifier to have a controller set"))?
                .try_into()?,
            duration: value
                .duration
                .duration
                .as_ref()
                .ok_or_else(|| anyhow!("Expected duration to have a duration specified"))
                .map(EffectDuration::from)?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ModifyCreature {
    ModifyBasePowerToughness(ModifyBasePowerToughness),
    ModifyCreatureTypes(ModifyCreatureTypes),
    AddPowerToughness(AddPowerToughness),
}

impl TryFrom<&protogen::effects::modify_creature::Modifier> for ModifyCreature {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::modify_creature::Modifier) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::modify_creature::Modifier::ModifyBasePowerToughness(modifier) => {
                Ok(Self::ModifyBasePowerToughness(modifier.try_into()?))
            }
            protogen::effects::modify_creature::Modifier::ModifyCreatureTypes(modifier) => {
                Ok(Self::ModifyCreatureTypes(modifier.try_into()?))
            }
            protogen::effects::modify_creature::Modifier::AddPowerToughness(modifier) => {
                Ok(Self::AddPowerToughness(modifier.try_into()?))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum SpellEffect {
    CounterSpell { target: SpellTarget },
    GainMana { mana: GainMana },
    BattlefieldModifier(BattlefieldModifier),
    ControllerDrawCards(usize),
    AddPowerToughness(AddPowerToughness),
    ModifyCreature(ModifyCreature),
}

impl TryFrom<&protogen::effects::spell_effect::Effect> for SpellEffect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::spell_effect::Effect) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::spell_effect::Effect::CounterSpell(counter) => {
                Ok(Self::CounterSpell {
                    target: counter.target.as_ref().unwrap_or_default().try_into()?,
                })
            }
            protogen::effects::spell_effect::Effect::GainMana(gain) => Ok(Self::GainMana {
                mana: gain
                    .gain
                    .as_ref()
                    .ok_or_else(|| anyhow!("Expected mana gain to have a gain field"))
                    .and_then(GainMana::try_from)?,
            }),
            protogen::effects::spell_effect::Effect::BattlefieldModifier(modifier) => {
                Ok(Self::BattlefieldModifier(modifier.try_into()?))
            }
            protogen::effects::spell_effect::Effect::ControllerDrawCards(draw) => {
                Ok(Self::ControllerDrawCards(usize::try_from(draw.count)?))
            }
            protogen::effects::spell_effect::Effect::AddPowerToughness(modifier) => {
                Ok(Self::AddPowerToughness(modifier.try_into()?))
            }
            protogen::effects::spell_effect::Effect::ModifyCreature(modifier) => {
                Ok(Self::ModifyCreature(
                    modifier
                        .modifier
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected modifier to have a modifer specified"))?
                        .try_into()?,
                ))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ActivatedAbilityEffect {
    CounterSpell { target: SpellTarget },
    GainMana { mana: GainMana },
    BattlefieldModifier(BattlefieldModifier),
    ControllerDrawCards(usize),
    Equip(ModifyCreature),
    AddPowerToughness(AddPowerToughness),
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
                    mana: gain
                        .gain
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected mana gain to have a gain field"))
                        .and_then(GainMana::try_from)?,
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
                        .modifier
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected equipment to have a modifier"))?
                        .modifier
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected modifier to have a modifier set"))?
                        .try_into()?,
                ))
            }
            protogen::effects::activated_ability_effect::Effect::AddPowerToughness(modifier) => {
                Ok(Self::AddPowerToughness(modifier.try_into()?))
            }
        }
    }
}
