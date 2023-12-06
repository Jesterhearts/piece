use anyhow::anyhow;
use enumset::{EnumSet, EnumSetType};

use crate::{
    card::Color,
    controller::Controller,
    mana::Mana,
    protogen,
    targets::{self, SpellTarget},
    types::{Subtype, Type},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mill {
    pub count: usize,
    pub target: Controller,
}

impl TryFrom<&protogen::effects::Mill> for Mill {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Mill) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
            target: value.target.get_or_default().try_into()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnFromGraveyardToLibrary {
    pub count: usize,
    pub controller: Controller,
    pub types: EnumSet<Type>,
}

impl TryFrom<&protogen::effects::ReturnFromGraveyardToLibrary> for ReturnFromGraveyardToLibrary {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::ReturnFromGraveyardToLibrary,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
            controller: value.controller.get_or_default().try_into()?,
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnFromGraveyardToBattlefield {
    pub count: usize,
    pub types: EnumSet<Type>,
}

impl TryFrom<&protogen::effects::ReturnFromGraveyardToBattlefield>
    for ReturnFromGraveyardToBattlefield
{
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::ReturnFromGraveyardToBattlefield,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, EnumSetType)]
pub enum Restriction {
    ControllerControlsBlackOrGreen,
}

impl TryFrom<&protogen::effects::Restriction> for Restriction {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Restriction) -> Result<Self, Self::Error> {
        value
            .restriction
            .as_ref()
            .ok_or_else(|| anyhow!("Expected restriction to have a restriction set"))
            .map(Restriction::from)
    }
}

impl From<&protogen::effects::restriction::Restriction> for Restriction {
    fn from(value: &protogen::effects::restriction::Restriction) -> Self {
        match value {
            protogen::effects::restriction::Restriction::ControllerControlsBlackOrGreen(_) => {
                Self::ControllerControlsBlackOrGreen
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EffectDuration {
    UntilEndOfTurn,
    UntilSourceLeavesBattlefield,
}

impl From<&protogen::effects::duration::Duration> for EffectDuration {
    fn from(value: &protogen::effects::duration::Duration) -> Self {
        match value {
            protogen::effects::duration::Duration::UntilEndOfTurn(_) => Self::UntilEndOfTurn,
            protogen::effects::duration::Duration::UntilSourceLeavesBattlefield(_) => {
                Self::UntilSourceLeavesBattlefield
            }
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
    pub targets: EnumSet<Subtype>,
    pub power: i32,
    pub toughness: i32,
}

impl TryFrom<&protogen::effects::ModifyBasePowerToughness> for ModifyBasePowerToughness {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ModifyBasePowerToughness) -> Result<Self, Self::Error> {
        Ok(Self {
            targets: value
                .targets
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
            power: value.power,
            toughness: value.toughness,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AddCreatureSubtypes {
    pub targets: EnumSet<Subtype>,
    pub types: EnumSet<Subtype>,
}

impl TryFrom<&protogen::effects::ModifyCreatureTypes> for AddCreatureSubtypes {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ModifyCreatureTypes) -> Result<Self, Self::Error> {
        Ok(Self {
            targets: value
                .targets
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
            types: value
                .types
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RemoveAllSubtypes {}

impl TryFrom<&protogen::effects::RemoveAllSubtypes> for RemoveAllSubtypes {
    type Error = anyhow::Error;

    fn try_from(_value: &protogen::effects::RemoveAllSubtypes) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AddPowerToughness {
    pub power: i32,
    pub toughness: i32,
}

impl TryFrom<&protogen::effects::AddPowerToughness> for AddPowerToughness {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::AddPowerToughness) -> Result<Self, Self::Error> {
        Ok(Self {
            power: value.power,
            toughness: value.toughness,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ModifyBattlefield {
    ModifyBasePowerToughness(ModifyBasePowerToughness),
    AddCreatureSubtypes(AddCreatureSubtypes),
    RemoveAllSubtypes(RemoveAllSubtypes),
    RemoveAllAbilities,
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
    pub restrictions: EnumSet<targets::Restriction>,
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
            restrictions: value
                .restrictions
                .iter()
                .map(targets::Restriction::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SpellEffect {
    CounterSpell { target: SpellTarget },
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
                    target: counter.target.as_ref().unwrap_or_default().try_into()?,
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
    CounterSpell { target: SpellTarget },
    GainMana { mana: GainMana },
    BattlefieldModifier(BattlefieldModifier),
    ControllerDrawCards(usize),
    Equip(Vec<ModifyBattlefield>),
    AddPowerToughnessToTarget(AddPowerToughness),
}

impl ActivatedAbilityEffect {
    pub fn wants_targets(&self) -> usize {
        match self {
            ActivatedAbilityEffect::CounterSpell { .. } => 1,
            ActivatedAbilityEffect::GainMana { .. } => 0,
            ActivatedAbilityEffect::BattlefieldModifier(_) => 0,
            ActivatedAbilityEffect::ControllerDrawCards(_) => 0,
            ActivatedAbilityEffect::Equip(_) => 1,
            ActivatedAbilityEffect::AddPowerToughnessToTarget(_) => 1,
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
            protogen::effects::activated_ability_effect::Effect::AddPowerToughnessToTarget(
                modifier,
            ) => Ok(Self::AddPowerToughnessToTarget(modifier.try_into()?)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenCreature {
    pub name: String,
    pub types: EnumSet<Type>,
    pub subtypes: EnumSet<Subtype>,
    pub colors: EnumSet<Color>,
    pub power: usize,
    pub toughness: usize,
}

impl TryFrom<&protogen::effects::create_token::Creature> for TokenCreature {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::create_token::Creature) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.clone(),
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .chain(std::iter::once(Ok(Type::Creature)))
                .collect::<anyhow::Result<EnumSet<_>>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
            colors: value
                .colors
                .iter()
                .map(Color::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
            power: usize::try_from(value.power)?,
            toughness: usize::try_from(value.toughness)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Creature(TokenCreature),
}

impl TryFrom<&protogen::effects::CreateToken> for Token {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::CreateToken) -> Result<Self, Self::Error> {
        value
            .token
            .as_ref()
            .ok_or_else(|| anyhow!("Expected CreateToken to have a token specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::effects::create_token::Token> for Token {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::create_token::Token) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::create_token::Token::Creature(creature) => {
                Ok(Self::Creature(creature.try_into()?))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggeredEffect {
    CreateToken(Token),
}

impl TryFrom<&protogen::effects::TriggeredEffect> for TriggeredEffect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::TriggeredEffect) -> Result<Self, Self::Error> {
        value
            .effect
            .as_ref()
            .ok_or_else(|| anyhow!("Expected triggered effect to havev an effect set"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::effects::triggered_effect::Effect> for TriggeredEffect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::triggered_effect::Effect) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::triggered_effect::Effect::CreateToken(token) => {
                Ok(Self::CreateToken(token.try_into()?))
            }
        }
    }
}
