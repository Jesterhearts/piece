use anyhow::anyhow;
use enumset::EnumSet;

use crate::{
    controller::Controller,
    mana::Mana,
    protogen,
    targets::{Restriction, SpellTarget},
    types::Subtype,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EffectDuration {
    UntilEndOfTurn,
    UntilSourceLeavesBattlefield,
    UntilUnattached,
}

impl From<&protogen::effects::duration::Duration> for EffectDuration {
    fn from(value: &protogen::effects::duration::Duration) -> Self {
        match value {
            protogen::effects::duration::Duration::UntilEndOfTurn(_) => Self::UntilEndOfTurn,
            protogen::effects::duration::Duration::UntilSourceLeavesBattlefield(_) => {
                Self::UntilSourceLeavesBattlefield
            }
            protogen::effects::duration::Duration::UntilUnattached(_) => Self::UntilUnattached,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Vigilance {
    pub restrictions: EnumSet<Restriction>,
}

impl TryFrom<&protogen::effects::Vigilance> for Vigilance {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Vigilance) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GainMana {
    Specific { gains: Vec<Mana> },
    Choice { choices: Vec<Vec<Mana>> },
}

impl TryFrom<&protogen::effects::GainMana> for GainMana {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::GainMana) -> Result<Self, Self::Error> {
        value
            .gain
            .as_ref()
            .ok_or_else(|| anyhow!("Expected mana gain to have a gain field"))
            .and_then(GainMana::try_from)
    }
}

impl TryFrom<&protogen::effects::gain_mana::Gain> for GainMana {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::gain_mana::Gain) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::gain_mana::Gain::Specific(specific) => Ok(Self::Specific {
                gains: specific
                    .gains
                    .iter()
                    .map(Mana::try_from)
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
                            .map(Mana::try_from)
                            .collect::<anyhow::Result<Vec<_>>>()
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?,
            }),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ModifyBasePowerToughness {
    pub targets: Vec<Subtype>,
    pub power: i32,
    pub toughness: i32,
    pub restrictions: EnumSet<Restriction>,
}

impl TryFrom<&protogen::effects::ModifyBasePowerToughness> for ModifyBasePowerToughness {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ModifyBasePowerToughness) -> Result<Self, Self::Error> {
        Ok(Self {
            targets: value
                .targets
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            power: value.power,
            toughness: value.toughness,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AddCreatureSubtypes {
    pub targets: Vec<Subtype>,
    pub types: Vec<Subtype>,
    pub restrictions: EnumSet<Restriction>,
}

impl TryFrom<&protogen::effects::ModifyCreatureTypes> for AddCreatureSubtypes {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ModifyCreatureTypes) -> Result<Self, Self::Error> {
        Ok(Self {
            targets: value
                .targets
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            types: value
                .types
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RemoveAllSubtypes {
    pub restrictions: EnumSet<Restriction>,
}

impl TryFrom<&protogen::effects::RemoveAllSubtypes> for RemoveAllSubtypes {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::RemoveAllSubtypes) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AddPowerToughness {
    pub power: i32,
    pub toughness: i32,
    pub restrictions: EnumSet<Restriction>,
}

impl TryFrom<&protogen::effects::AddPowerToughness> for AddPowerToughness {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::AddPowerToughness) -> Result<Self, Self::Error> {
        Ok(Self {
            power: value.power,
            toughness: value.toughness,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ModifyBattlefield {
    ModifyBasePowerToughness(ModifyBasePowerToughness),
    AddCreatureSubtypes(AddCreatureSubtypes),
    RemoveAllSubtypes(RemoveAllSubtypes),
    AddPowerToughness(AddPowerToughness),
    Vigilance(Vigilance),
}

impl TryFrom<&protogen::effects::ModifyBattlefield> for ModifyBattlefield {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ModifyBattlefield) -> Result<Self, Self::Error> {
        value
            .modifier
            .as_ref()
            .ok_or_else(|| anyhow!("Expected modifier to have a modifier set"))
            .and_then(Self::try_from)
    }
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
                Ok(Self::AddCreatureSubtypes(modifier.try_into()?))
            }
            protogen::effects::modify_battlefield::Modifier::AddPowerToughness(modifier) => {
                Ok(Self::AddPowerToughness(modifier.try_into()?))
            }
            protogen::effects::modify_battlefield::Modifier::RemoveAllSubtypes(modifier) => {
                Ok(Self::RemoveAllSubtypes(modifier.try_into()?))
            }
            protogen::effects::modify_battlefield::Modifier::Vigilance(vigilance) => {
                Ok(Self::Vigilance(vigilance.try_into()?))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BattlefieldModifier {
    pub modifier: ModifyBattlefield,
    pub controller: Controller,
    pub duration: EffectDuration,
}
impl BattlefieldModifier {
    pub fn restrictions(&self) -> &EnumSet<Restriction> {
        match &self.modifier {
            ModifyBattlefield::ModifyBasePowerToughness(ModifyBasePowerToughness {
                restrictions,
                ..
            })
            | ModifyBattlefield::AddCreatureSubtypes(AddCreatureSubtypes {
                restrictions, ..
            })
            | ModifyBattlefield::RemoveAllSubtypes(RemoveAllSubtypes { restrictions })
            | ModifyBattlefield::AddPowerToughness(AddPowerToughness { restrictions, .. })
            | ModifyBattlefield::Vigilance(Vigilance { restrictions }) => restrictions,
        }
    }
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SpellEffect {
    CounterSpell { valid_target: SpellTarget },
    GainMana { mana: GainMana },
    BattlefieldModifier(BattlefieldModifier),
    ControllerDrawCards(usize),
    AddPowerToughnessToTarget(AddPowerToughness),
    ModifyCreature(BattlefieldModifier),
    ExileTargetCreature,
    ExileTargetCreatureManifestTopOfLibrary,
}

impl TryFrom<&protogen::effects::SpellEffect> for SpellEffect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::SpellEffect) -> Result<Self, Self::Error> {
        value
            .effect
            .as_ref()
            .ok_or_else(|| anyhow!("Expected effect to have an effect specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::effects::spell_effect::Effect> for SpellEffect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::spell_effect::Effect) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::spell_effect::Effect::CounterSpell(counter) => {
                Ok(Self::CounterSpell {
                    valid_target: counter
                        .valid_target
                        .as_ref()
                        .unwrap_or_default()
                        .try_into()?,
                })
            }
            protogen::effects::spell_effect::Effect::GainMana(gain) => Ok(Self::GainMana {
                mana: GainMana::try_from(gain)?,
            }),
            protogen::effects::spell_effect::Effect::BattlefieldModifier(modifier) => {
                Ok(Self::BattlefieldModifier(modifier.try_into()?))
            }
            protogen::effects::spell_effect::Effect::ControllerDrawCards(draw) => {
                Ok(Self::ControllerDrawCards(usize::try_from(draw.count)?))
            }
            protogen::effects::spell_effect::Effect::AddPowerToughnessToTarget(modifier) => {
                Ok(Self::AddPowerToughnessToTarget(modifier.try_into()?))
            }
            protogen::effects::spell_effect::Effect::ModifyCreature(modifier) => {
                Ok(Self::ModifyCreature(modifier.try_into()?))
            }
            protogen::effects::spell_effect::Effect::ExileTargetCreature(_) => {
                Ok(Self::ExileTargetCreature)
            }
            protogen::effects::spell_effect::Effect::ExileTargetCreatureManifestTopOfLibrary(_) => {
                Ok(Self::ExileTargetCreatureManifestTopOfLibrary)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ActivatedAbilityEffect {
    CounterSpell { valid_target: SpellTarget },
    GainMana { mana: GainMana },
    BattlefieldModifier(BattlefieldModifier),
    ControllerDrawCards(usize),
    Equip(Vec<ModifyBattlefield>),
    AddPowerToughnessToTarget(AddPowerToughness),
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
                    valid_target: counter
                        .valid_target
                        .as_ref()
                        .unwrap_or_default()
                        .try_into()?,
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
            protogen::effects::activated_ability_effect::Effect::AddPowerToughnessToTarget(
                modifier,
            ) => Ok(Self::AddPowerToughnessToTarget(modifier.try_into()?)),
        }
    }
}
