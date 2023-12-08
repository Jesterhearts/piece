use std::collections::HashSet;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::{
    card::Color,
    controller::Controller,
    mana::Mana,
    protogen,
    targets::{self, SpellTarget},
    types::{Subtype, Type},
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Destination {
    Hand,
    TopOfLibrary,
    Battlefield,
}

impl TryFrom<&protogen::effects::Destination> for Destination {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Destination) -> Result<Self, Self::Error> {
        value
            .destination
            .as_ref()
            .ok_or_else(|| anyhow!("Expected destination to have a destination set"))
            .map(Self::from)
    }
}

impl From<&protogen::effects::destination::Destination> for Destination {
    fn from(value: &protogen::effects::destination::Destination) -> Self {
        match value {
            protogen::effects::destination::Destination::Hand(_) => Self::Hand,
            protogen::effects::destination::Destination::TopOfLibrary(_) => Self::TopOfLibrary,
            protogen::effects::destination::Destination::Battlefield(_) => Self::Battlefield,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TutorLibrary {
    pub restrictions: Vec<targets::Restriction>,
    pub destination: Destination,
    pub reveal: bool,
}

impl TryFrom<&protogen::effects::TutorLibrary> for TutorLibrary {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::TutorLibrary) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(targets::Restriction::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            destination: value.destination.get_or_default().try_into()?,
            reveal: value.reveal,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ReturnFromGraveyardToLibrary {
    pub count: usize,
    pub controller: Controller,
    pub types: HashSet<Type>,
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
                .collect::<anyhow::Result<HashSet<_>>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ReturnFromGraveyardToBattlefield {
    pub count: usize,
    pub types: HashSet<Type>,
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
                .collect::<anyhow::Result<HashSet<_>>>()?,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy)]
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Default)]
pub struct ModifyBattlefield {
    pub base_power: Option<i32>,
    pub base_toughness: Option<i32>,

    pub add_power: Option<i32>,
    pub add_toughness: Option<i32>,

    pub add_types: HashSet<Type>,
    pub add_subtypes: HashSet<Subtype>,

    pub remove_all_subtypes: bool,
    pub remove_all_abilities: bool,

    pub entire_battlefield: bool,
    pub global: bool,

    pub add_vigilance: bool,
}

impl TryFrom<&protogen::effects::ModifyBattlefield> for ModifyBattlefield {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ModifyBattlefield) -> Result<Self, Self::Error> {
        Ok(Self {
            base_power: value.base_power,
            base_toughness: value.base_toughness,
            add_power: value.add_power,
            add_toughness: value.add_toughness,
            add_types: value
                .add_types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            add_subtypes: value
                .add_subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            remove_all_subtypes: value.remove_all_subtypes,
            remove_all_abilities: false,
            entire_battlefield: value.entire_battlefield,
            global: value.global,
            add_vigilance: value.add_vigilance,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct BattlefieldModifier {
    pub modifier: ModifyBattlefield,
    pub controller: Controller,
    pub duration: EffectDuration,
    pub restrictions: Vec<targets::Restriction>,
}

impl TryFrom<&protogen::effects::BattlefieldModifier> for BattlefieldModifier {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::BattlefieldModifier) -> Result<Self, Self::Error> {
        Ok(Self {
            modifier: value.modifier.get_or_default().try_into()?,
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
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum SpellEffect {
    CounterSpell { target: SpellTarget },
    GainMana { mana: GainMana },
    BattlefieldModifier(BattlefieldModifier),
    ControllerDrawCards(usize),
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TokenCreature {
    pub name: String,
    pub types: HashSet<Type>,
    pub subtypes: HashSet<Subtype>,
    pub colors: HashSet<Color>,
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
                .collect::<anyhow::Result<HashSet<_>>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            colors: value
                .colors
                .iter()
                .map(Color::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            power: usize::try_from(value.power)?,
            toughness: usize::try_from(value.toughness)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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
