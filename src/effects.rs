use std::{collections::HashSet, str::FromStr};

use anyhow::{anyhow, Context};
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    abilities::{ActivatedAbility, GainManaAbility},
    card::{Color, Keyword},
    controller::ControllerRestriction,
    in_play::{Database, ReplacementEffectId},
    newtype_enum::newtype_enum,
    player::{AllPlayers, Controller},
    protogen,
    stack::ActiveTarget,
    targets::{Restriction, SpellTarget},
    types::{Subtype, Type},
    Battlefield,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Hand,
    TopOfLibrary,
    Battlefield { enters_tapped: bool },
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
            protogen::effects::destination::Destination::Battlefield(battlefield) => {
                Self::Battlefield {
                    enters_tapped: battlefield.enters_tapped,
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mill {
    pub count: usize,
    pub target: ControllerRestriction,
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
    pub controller: ControllerRestriction,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

newtype_enum! {
#[derive(Debug, PartialEq, Eq, Clone, Copy, bevy_ecs::component::Component)]
pub enum EffectDuration {
    UntilEndOfTurn,
    UntilSourceLeavesBattlefield,
}
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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Component)]
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

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ModifyBattlefield {
    pub base_power: Option<i32>,
    pub base_toughness: Option<i32>,

    pub add_power: Option<i32>,
    pub add_toughness: Option<i32>,

    pub dynamic_power_toughness: Option<DynamicPowerToughness>,

    pub add_types: HashSet<Type>,
    pub add_subtypes: HashSet<Subtype>,

    pub remove_types: HashSet<Type>,
    pub remove_subtypes: HashSet<Subtype>,

    pub add_ability: Option<ActivatedAbility>,
    pub mana_ability: Option<GainManaAbility>,

    pub remove_all_subtypes: bool,
    pub remove_all_abilities: bool,

    pub entire_battlefield: bool,
    pub global: bool,

    pub add_keywords: HashSet<Keyword>,
    pub remove_keywords: HashSet<Keyword>,
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
            remove_types: value
                .remove_types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<_>>()?,
            remove_subtypes: value
                .remove_subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<_>>()?,
            add_ability: value
                .add_ability
                .as_ref()
                .map_or(Ok(None), |v| v.try_into().map(Some))?,
            mana_ability: value
                .mana_ability
                .as_ref()
                .map_or(Ok(None), |v| v.try_into().map(Some))?,
            remove_all_subtypes: value.remove_all_subtypes,
            remove_all_abilities: false,
            entire_battlefield: value.entire_battlefield,
            global: value.global,
            add_keywords: value
                .add_keywords
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| Keyword::from_str(s.trim()).with_context(|| anyhow!("Parsing {}", s)))
                .collect::<anyhow::Result<_>>()?,
            remove_keywords: value
                .remove_keywords
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| Keyword::from_str(s.trim()).with_context(|| anyhow!("Parsing {}", s)))
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BattlefieldModifier {
    pub modifier: ModifyBattlefield,
    pub controller: ControllerRestriction,
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

#[derive(Debug, PartialEq, Eq, Clone)]
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

pub mod reveal_each_top_of_library {
    use crate::{effects::Effect, protogen, targets::Restriction};

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct ForEach {
        pub restrictions: Vec<Restriction>,
        pub effects: Vec<Effect>,
        pub if_none: Vec<Effect>,
    }

    impl TryFrom<&protogen::effects::reveal_each_top_of_library::ForEach> for ForEach {
        type Error = anyhow::Error;

        fn try_from(
            value: &protogen::effects::reveal_each_top_of_library::ForEach,
        ) -> Result<Self, Self::Error> {
            Ok(Self {
                restrictions: value
                    .restrictions
                    .iter()
                    .map(Restriction::try_from)
                    .collect::<anyhow::Result<_>>()?,
                effects: value
                    .effects
                    .iter()
                    .map(Effect::try_from)
                    .collect::<anyhow::Result<_>>()?,
                if_none: value
                    .if_none
                    .effects
                    .iter()
                    .map(Effect::try_from)
                    .collect::<anyhow::Result<_>>()?,
            })
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RevealEachTopOfLibrary {
    pub for_each: reveal_each_top_of_library::ForEach,
}

impl TryFrom<&protogen::effects::RevealEachTopOfLibrary> for RevealEachTopOfLibrary {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::RevealEachTopOfLibrary) -> Result<Self, Self::Error> {
        Ok(Self {
            for_each: value.for_each.get_or_default().try_into()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Component)]
pub enum Effect {
    BattlefieldModifier(BattlefieldModifier),
    ControllerDrawCards(usize),
    ControllerLosesLife(usize),
    CopyOfAnyCreatureNonTargeting,
    CounterSpell { target: SpellTarget },
    CreateToken(Token),
    CreateTokenCopy { modifiers: Vec<ModifyBattlefield> },
    ReturnSelfToHand,
    RevealEachTopOfLibrary(RevealEachTopOfLibrary),
    DealDamage(DealDamage),
    Equip(Vec<ModifyBattlefield>),
    ExileTargetCreature,
    ExileTargetCreatureManifestTopOfLibrary,
    GainCounter(Counter),
    Mill(Mill),
    ModifyCreature(BattlefieldModifier),
    ReturnFromGraveyardToBattlefield(ReturnFromGraveyardToBattlefield),
    ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary),
    TutorLibrary(TutorLibrary),
}

impl Effect {
    pub fn choices(
        &self,
        db: &mut Database,
        all_players: &AllPlayers,
        targets: &[ActiveTarget],
    ) -> Vec<String> {
        targets
            .iter()
            .map(|target| target.display(db, all_players))
            .collect_vec()
    }

    pub fn wants_targets(&self) -> usize {
        match self {
            Effect::BattlefieldModifier(_) => 0,
            Effect::ControllerDrawCards(_) => 0,
            Effect::CounterSpell { .. } => 1,
            Effect::CreateToken(_) => 0,
            Effect::DealDamage(_) => 1,
            Effect::Equip(_) => 1,
            Effect::ExileTargetCreature => 1,
            Effect::ExileTargetCreatureManifestTopOfLibrary => 1,
            Effect::GainCounter(_) => 0,
            Effect::ModifyCreature(_) => 1,
            Effect::ControllerLosesLife(_) => 0,
            Effect::CopyOfAnyCreatureNonTargeting => 1,
            Effect::Mill(_) => 1,
            Effect::ReturnFromGraveyardToBattlefield(ReturnFromGraveyardToBattlefield {
                count,
                ..
            }) => *count,
            Effect::ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary {
                count, ..
            }) => *count,
            Effect::TutorLibrary(_) => 0,
            Effect::CreateTokenCopy { .. } => 1,
            Effect::ReturnSelfToHand => 0,
            Effect::RevealEachTopOfLibrary(_) => 0,
        }
    }
}

impl TryFrom<&protogen::effects::Effect> for Effect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Effect) -> Result<Self, Self::Error> {
        value
            .effect
            .as_ref()
            .ok_or_else(|| anyhow!("Expected effect to have an effect specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::effects::effect::Effect> for Effect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::effect::Effect) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::effect::Effect::CounterSpell(counter) => Ok(Self::CounterSpell {
                target: counter
                    .valid_target
                    .as_ref()
                    .unwrap_or_default()
                    .try_into()?,
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
            protogen::effects::effect::Effect::ControllerLosesLife(value) => {
                Ok(Self::ControllerLosesLife(usize::try_from(value.count)?))
            }
            protogen::effects::effect::Effect::CopyOfAnyCreatureNonTargeting(_) => {
                Ok(Self::CopyOfAnyCreatureNonTargeting)
            }
            protogen::effects::effect::Effect::Mill(mill) => Ok(Self::Mill(mill.try_into()?)),
            protogen::effects::effect::Effect::ReturnFromGraveyardToBattlefield(ret) => {
                Ok(Self::ReturnFromGraveyardToBattlefield(ret.try_into()?))
            }
            protogen::effects::effect::Effect::ReturnFromGraveyardToLibrary(ret) => {
                Ok(Self::ReturnFromGraveyardToLibrary(ret.try_into()?))
            }
            protogen::effects::effect::Effect::TutorLibrary(tutor) => {
                Ok(Self::TutorLibrary(tutor.try_into()?))
            }
            protogen::effects::effect::Effect::CreateTokenCopy(copy) => Ok(Self::CreateTokenCopy {
                modifiers: copy
                    .modifiers
                    .iter()
                    .map(ModifyBattlefield::try_from)
                    .collect::<anyhow::Result<_>>()?,
            }),
            protogen::effects::effect::Effect::ReturnSelfToHand(_) => Ok(Self::ReturnSelfToHand),
            protogen::effects::effect::Effect::RevealEachTopOfLibrary(reveal) => {
                Ok(Self::RevealEachTopOfLibrary(reveal.try_into()?))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deref, DerefMut, Component, Default)]
pub struct Effects(pub Vec<AnyEffect>);

#[derive(Debug, PartialEq, Eq, Clone)]
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
    pub fn effect(&self, db: &mut Database, controller: Controller) -> &Effect {
        if self.threshold.is_some()
            && Battlefield::number_of_cards_in_graveyard(db, controller) >= 7
        {
            self.threshold.as_ref().unwrap()
        } else {
            &self.effect
        }
    }

    pub fn into_effect(self, db: &mut Database, controller: Controller) -> Effect {
        if self.threshold.is_some()
            && Battlefield::number_of_cards_in_graveyard(db, controller) >= 7
        {
            self.threshold.unwrap()
        } else {
            self.effect
        }
    }

    pub fn wants_targets(&self, db: &mut Database, controller: Controller) -> usize {
        let effect = self.effect(db, controller);
        effect.wants_targets()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenCreature {
    pub name: String,
    pub types: HashSet<Type>,
    pub subtypes: HashSet<Subtype>,
    pub colors: HashSet<Color>,
    pub keywords: HashSet<Keyword>,
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
            keywords: value
                .keywords
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| Keyword::from_str(s.trim()).with_context(|| anyhow!("Parsing {}", s)))
                .collect::<anyhow::Result<_>>()?,
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

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
#[derive(strum::EnumIter)]
pub enum Counter {
    Charge,
    P1P1,
    M1M1,
}
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

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
pub enum Replacing {
    Draw,
    Etb,
}
}

impl From<&protogen::effects::replacement_effect::Replacing> for Replacing {
    fn from(value: &protogen::effects::replacement_effect::Replacing) -> Self {
        match value {
            protogen::effects::replacement_effect::Replacing::Draw(_) => Self::Draw,
            protogen::effects::replacement_effect::Replacing::Etb(_) => Self::Etb,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct ReplacementEffects(pub Vec<ReplacementEffectId>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacementEffect {
    pub replacing: Replacing,
    pub restrictions: Vec<Restriction>,
    pub effects: Vec<AnyEffect>,
}

impl TryFrom<&protogen::effects::ReplacementEffect> for ReplacementEffect {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ReplacementEffect) -> Result<Self, Self::Error> {
        Ok(Self {
            replacing: value
                .replacing
                .as_ref()
                .ok_or_else(|| {
                    anyhow!("Expected replacement effect to have a replacement specified")
                })
                .map(Replacing::from)?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}
