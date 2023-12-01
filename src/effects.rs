use anyhow::anyhow;

use crate::{mana::Mana, protogen, targets::SpellTarget, types::Subtype};

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

impl TryFrom<&protogen::effects::effect::gain_mana::Gain> for GainMana {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::effect::gain_mana::Gain) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::effect::gain_mana::Gain::Specific(specific) => Ok(Self::Specific {
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
            protogen::effects::effect::gain_mana::Gain::Choice(choice) => Ok(Self::Choice {
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
pub enum ModifyBattlefield {
    ModifyBasePowerToughness {
        targets: Vec<Subtype>,
        power: usize,
        toughness: usize,
        duration: EffectDuration,
    },
}

impl TryFrom<&protogen::effects::effect::modify_battlefield::Modifier> for ModifyBattlefield {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::effect::modify_battlefield::Modifier,
    ) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::effect::modify_battlefield::Modifier::ModifyBasePowerToughness(
                modifier,
            ) => Ok(Self::ModifyBasePowerToughness {
                targets: modifier
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
                power: usize::try_from(modifier.power)?,
                toughness: usize::try_from(modifier.toughness)?,
                duration: modifier
                    .duration
                    .duration
                    .as_ref()
                    .ok_or_else(|| anyhow!("Expected duration to have a duration specified"))
                    .map(EffectDuration::from)?,
            }),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Effect {
    CounterSpell { target: SpellTarget },
    GainMana { mana: GainMana },
    ModifyBattlefield(ModifyBattlefield),
    ControllerDrawCards(usize),
}

impl TryFrom<&protogen::effects::effect::Effect> for Effect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::effect::Effect) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::effect::Effect::CounterSpell(counter) => Ok(Self::CounterSpell {
                target: counter
                    .target
                    .as_ref()
                    .ok_or_else(|| anyhow!("Expected counterspell to have a target"))?
                    .try_into()?,
            }),
            protogen::effects::effect::Effect::GainMana(gain) => Ok(Self::GainMana {
                mana: gain
                    .gain
                    .as_ref()
                    .ok_or_else(|| anyhow!("Expected mana gain to have a gain field"))
                    .and_then(GainMana::try_from)?,
            }),
            protogen::effects::effect::Effect::ModifyBattlefield(modifier) => {
                Ok(Self::ModifyBattlefield(ModifyBattlefield::try_from(
                    modifier
                        .modifier
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected modifier to have a modifier set"))?,
                )?))
            }
            protogen::effects::effect::Effect::ControllerDrawCards(draw) => {
                Ok(Self::ControllerDrawCards(usize::try_from(draw.count)?))
            }
        }
    }
}
