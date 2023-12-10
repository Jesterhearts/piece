use std::collections::HashSet;

use anyhow::anyhow;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    abilities::ActivatedAbility,
    battlefield::Battlefield,
    card::Color,
    controller::Controller,
    mana::Mana,
    player::PlayerId,
    protogen,
    targets::{Restriction, SpellTarget},
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
    pub restrictions: Vec<Restriction>,
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
                .map(Restriction::try_from)
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum DynamicPowerToughness {
    NumberOfCountersOnThis(Counter),
}

impl TryFrom<&protogen::effects::DynamicPowerToughness> for DynamicPowerToughness {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::DynamicPowerToughness) -> Result<Self, Self::Error> {
        value
            .source
            .as_ref()
            .ok_or_else(|| anyhow!("Expected dynamic p/t to have a source set"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::effects::dynamic_power_toughness::Source> for DynamicPowerToughness {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::dynamic_power_toughness::Source,
    ) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::dynamic_power_toughness::Source::NumberOfCountersOnThis(counter) => {
                Ok(Self::NumberOfCountersOnThis(
                    counter.counter.get_or_default().try_into()?,
                ))
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Default)]
pub struct ModifyBattlefield {
    pub base_power: Option<i32>,
    pub base_toughness: Option<i32>,

    pub add_power: Option<i32>,
    pub add_toughness: Option<i32>,

    pub dynamic_power_toughness: Option<DynamicPowerToughness>,

    pub add_types: HashSet<Type>,
    pub add_subtypes: HashSet<Subtype>,

    pub add_ability: Option<ActivatedAbility>,

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
            dynamic_power_toughness: value
                .add_dynamic_power_toughness
                .as_ref()
                .map_or(Ok(None), |pt| pt.try_into().map(Some))?,
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
            add_ability: value
                .add_ability
                .as_ref()
                .map_or(Ok(None), |v| v.try_into().map(Some))?,
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
    pub restrictions: Vec<Restriction>,
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
                .map(Restriction::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct DealDamage {
    pub quantity: usize,
    pub restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::DealDamage> for DealDamage {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::DealDamage) -> Result<Self, Self::Error> {
        Ok(Self {
            quantity: usize::try_from(value.quantity)?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Effect {
    BattlefieldModifier(BattlefieldModifier),
    ControllerDrawCards(usize),
    CounterSpell { target: SpellTarget },
    CreateToken(Token),
    DealDamage(DealDamage),
    Equip(Vec<ModifyBattlefield>),
    ExileTargetCreature,
    ExileTargetCreatureManifestTopOfLibrary,
    GainCounter(Counter),
    GainMana { mana: GainMana },
    ModifyCreature(BattlefieldModifier),
}

impl TryFrom<&protogen::effects::effect::Effect> for Effect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::effect::Effect) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::effect::Effect::CounterSpell(counter) => Ok(Self::CounterSpell {
                target: counter.target.as_ref().unwrap_or_default().try_into()?,
            }),
            protogen::effects::effect::Effect::GainMana(gain) => Ok(Self::GainMana {
                mana: GainMana::try_from(gain)?,
            }),
            protogen::effects::effect::Effect::BattlefieldModifier(modifier) => {
                Ok(Self::BattlefieldModifier(modifier.try_into()?))
            }
            protogen::effects::effect::Effect::ControllerDrawCards(draw) => {
                Ok(Self::ControllerDrawCards(usize::try_from(draw.count)?))
            }
            protogen::effects::effect::Effect::ModifyCreature(modifier) => {
                Ok(Self::ModifyCreature(modifier.try_into()?))
            }
            protogen::effects::effect::Effect::ExileTargetCreature(_) => {
                Ok(Self::ExileTargetCreature)
            }
            protogen::effects::effect::Effect::ExileTargetCreatureManifestTopOfLibrary(_) => {
                Ok(Self::ExileTargetCreatureManifestTopOfLibrary)
            }
            protogen::effects::effect::Effect::DealDamage(dmg) => {
                Ok(Self::DealDamage(dmg.try_into()?))
            }
            protogen::effects::effect::Effect::CreateToken(token) => {
                Ok(Self::CreateToken(token.try_into()?))
            }
            protogen::effects::effect::Effect::Equip(equip) => Ok(Self::Equip(
                equip
                    .modifiers
                    .iter()
                    .map(ModifyBattlefield::try_from)
                    .collect::<anyhow::Result<_>>()?,
            )),
            protogen::effects::effect::Effect::GainCounter(counter) => Ok(Self::GainCounter(
                counter.counter.get_or_default().try_into()?,
            )),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct AnyEffect {
    pub effect: Effect,
    pub threshold: Option<Effect>,
}

impl TryFrom<&protogen::effects::Effect> for AnyEffect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Effect) -> Result<Self, Self::Error> {
        Ok(Self {
            effect: value
                .effect
                .as_ref()
                .ok_or_else(|| anyhow!("Expected effect to have an effect specified"))
                .and_then(Effect::try_from)?,
            threshold: value.threshold.as_ref().map_or(Ok(None), |threshold| {
                threshold
                    .effect
                    .as_ref()
                    .ok_or_else(|| anyhow!("Expected effect to have an effect specified"))
                    .and_then(Effect::try_from)
                    .map(Some)
            })?,
        })
    }
}

impl AnyEffect {
    pub fn effect(&self, db: &Connection, controller: PlayerId) -> anyhow::Result<&Effect> {
        if self.threshold.is_some()
            && Battlefield::number_of_cards_in_graveyard(db, controller)? >= 7
        {
            Ok(self.threshold.as_ref().unwrap())
        } else {
            Ok(&self.effect)
        }
    }

    pub fn into_effect(self, db: &Connection, controller: PlayerId) -> anyhow::Result<Effect> {
        if self.threshold.is_some()
            && Battlefield::number_of_cards_in_graveyard(db, controller)? >= 7
        {
            Ok(self.threshold.unwrap())
        } else {
            Ok(self.effect)
        }
    }

    pub(crate) fn wants_targets(
        &self,
        db: &Connection,
        controller: PlayerId,
    ) -> anyhow::Result<usize> {
        match self.effect(db, controller)? {
            Effect::BattlefieldModifier(_) => Ok(0),
            Effect::ControllerDrawCards(_) => Ok(0),
            Effect::CounterSpell { .. } => Ok(1),
            Effect::CreateToken(_) => Ok(0),
            Effect::DealDamage(_) => Ok(1),
            Effect::Equip(_) => Ok(1),
            Effect::ExileTargetCreature => Ok(1),
            Effect::ExileTargetCreatureManifestTopOfLibrary => Ok(1),
            Effect::GainCounter(_) => Ok(0),
            Effect::GainMana { .. } => Ok(0),
            Effect::ModifyCreature(_) => Ok(1),
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
pub enum Counter {
    Charge,
    P1P1,
    M1M1,
}

impl TryFrom<&protogen::counters::Counter> for Counter {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::counters::Counter) -> Result<Self, Self::Error> {
        value
            .type_
            .as_ref()
            .ok_or_else(|| anyhow!("Expected counter to have a type specified"))
            .map(Self::from)
    }
}

impl From<&protogen::counters::counter::Type> for Counter {
    fn from(value: &protogen::counters::counter::Type) -> Self {
        match value {
            protogen::counters::counter::Type::Charge(_) => Self::Charge,
            protogen::counters::counter::Type::P1p1(_) => Self::P1P1,
            protogen::counters::counter::Type::M1m1(_) => Self::M1M1,
        }
    }
}
