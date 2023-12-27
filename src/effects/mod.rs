pub mod battlefield_modifier;
pub mod cascade;
pub mod controller_draws_cards;
pub mod controller_loses_life;
pub mod copy_of_any_creature_non_targeting;
pub mod counter_spell;
pub mod create_token;
pub mod create_token_copy;
pub mod deal_damage;
pub mod destroy_each;
pub mod destroy_target;
pub mod discover;
pub mod equip;
pub mod exile_target;
pub mod exile_target_creature_manifest_top_of_library;
pub mod exile_target_graveyard;
pub mod foreach_mana_of_source;
pub mod gain_life;
pub mod mill;
pub mod modal;
pub mod modify_target;
pub mod multiply_tokens;
pub mod pay_cost_then;
pub mod return_from_graveyard_to_battlefield;
pub mod return_from_graveyard_to_hand;
pub mod return_from_graveyard_to_library;
pub mod return_self_to_hand;
pub mod return_target_to_hand;
pub mod return_transformed;
pub mod reveal_each_top_of_library;
pub mod scry;
pub mod self_explores;
pub mod target_controller_gains_tokens;
pub mod target_creature_explores;
pub mod target_gains_counters;
pub mod target_to_top_of_library;
pub mod transform;
pub mod tutor_library;
pub mod untap_target;
pub mod untap_this;

use std::{collections::HashSet, fmt::Debug, str::FromStr, vec::IntoIter};

use anyhow::{anyhow, Context};
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    abilities::{ActivatedAbility, GainManaAbility},
    battlefield::PendingResults,
    card::{Color, Keyword},
    controller::ControllerRestriction,
    effects::{
        cascade::Cascade,
        controller_draws_cards::ControllerDrawsCards,
        controller_loses_life::ControllerLosesLife,
        copy_of_any_creature_non_targeting::CopyOfAnyCreatureNonTargeting,
        counter_spell::CounterSpell,
        create_token::CreateToken,
        create_token_copy::CreateTokenCopy,
        deal_damage::DealDamage,
        destroy_each::DestroyEach,
        destroy_target::DestroyTarget,
        discover::Discover,
        equip::Equip,
        exile_target::ExileTarget,
        exile_target_creature_manifest_top_of_library::ExileTargetCreatureManifestTopOfLibrary,
        exile_target_graveyard::ExileTargetGraveyard,
        foreach_mana_of_source::ForEachManaOfSource,
        gain_life::GainLife,
        mill::Mill,
        modal::Modal,
        modify_target::ModifyTarget,
        multiply_tokens::MultiplyTokens,
        pay_cost_then::PayCostThen,
        return_from_graveyard_to_battlefield::ReturnFromGraveyardToBattlefield,
        return_from_graveyard_to_hand::ReturnFromGraveyardToHand,
        return_from_graveyard_to_library::ReturnFromGraveyardToLibrary,
        return_self_to_hand::ReturnSelfToHand,
        return_target_to_hand::ReturnTargetToHand,
        return_transformed::ReturnTransformed,
        reveal_each_top_of_library::RevealEachTopOfLibrary,
        scry::Scry,
        self_explores::SelfExplores,
        target_controller_gains_tokens::TargetControllerGainsTokens,
        target_creature_explores::TargetCreatureExplores,
        target_gains_counters::{Counter, TargetGainsCounters},
        target_to_top_of_library::TargetToTopOfLibrary,
        transform::Transform,
        tutor_library::TutorLibrary,
        untap_target::UntapTarget,
        untap_this::UntapThis,
    },
    in_play::{CardId, Database, ReplacementEffectId},
    newtype_enum::newtype_enum,
    player::{AllPlayers, Controller, Player},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
    types::{Subtype, Type},
    Battlefield,
};

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Effect(pub &'static (dyn EffectBehaviors + Send + Sync));

pub use battlefield_modifier::BattlefieldModifier;

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

newtype_enum! {
#[derive(Debug, PartialEq, Eq, Clone, Copy, bevy_ecs::component::Component)]
pub enum EffectDuration {
    Permanently,
    UntilEndOfTurn,
    UntilSourceLeavesBattlefield,
    UntilTargetLeavesBattlefield,
}
}

impl TryFrom<&protogen::effects::Duration> for EffectDuration {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Duration) -> Result<Self, Self::Error> {
        value
            .duration
            .as_ref()
            .ok_or_else(|| anyhow!("Expected duration to have a duration set"))
            .map(Self::from)
    }
}

impl From<&protogen::effects::duration::Duration> for EffectDuration {
    fn from(value: &protogen::effects::duration::Duration) -> Self {
        match value {
            protogen::effects::duration::Duration::UntilEndOfTurn(_) => Self::UntilEndOfTurn,
            protogen::effects::duration::Duration::UntilSourceLeavesBattlefield(_) => {
                Self::UntilSourceLeavesBattlefield
            }
            protogen::effects::duration::Duration::UntilTargetLeavesBattlefield(_) => {
                Self::UntilTargetLeavesBattlefield
            }
            protogen::effects::duration::Duration::Permanently(_) => Self::Permanently,
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

#[derive(Debug, Clone, Default)]
pub struct ModifyBattlefield {
    pub base_power: Option<i32>,
    pub base_toughness: Option<i32>,

    pub add_power: Option<i32>,
    pub add_toughness: Option<i32>,

    pub dynamic_power_toughness: Option<DynamicPowerToughness>,

    pub add_types: IndexSet<Type>,
    pub add_subtypes: IndexSet<Subtype>,

    pub remove_types: IndexSet<Type>,
    pub remove_subtypes: IndexSet<Subtype>,

    pub add_colors: HashSet<Color>,

    pub add_ability: Option<ActivatedAbility>,
    pub mana_ability: Option<GainManaAbility>,

    pub remove_all_subtypes: bool,
    pub remove_all_abilities: bool,
    pub remove_all_colors: bool,

    pub entire_battlefield: bool,
    pub global: bool,

    pub add_keywords: ::counter::Counter<Keyword>,
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
                .collect::<anyhow::Result<_>>()?,
            add_subtypes: value
                .add_subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<_>>()?,
            add_colors: value
                .add_colors
                .iter()
                .map(Color::try_from)
                .collect::<anyhow::Result<_>>()?,
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
            remove_all_colors: value.remove_all_colors,
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

pub trait EffectBehaviors: Debug {
    fn choices(
        &'static self,
        db: &mut Database,
        all_players: &AllPlayers,
        targets: &[ActiveTarget],
    ) -> Vec<String> {
        targets
            .iter()
            .map(|target| target.display(db, all_players))
            .collect_vec()
    }

    fn modes(&'static self) -> Vec<Mode> {
        vec![]
    }

    fn is_sorcery_speed(&'static self) -> bool {
        false
    }

    fn needs_targets(&'static self) -> usize;

    fn wants_targets(&'static self) -> usize;

    fn valid_targets(
        &'static self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        let _ = db;
        let _ = source;
        let _ = controller;
        let _ = already_chosen;
        vec![]
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    );

    fn push_behavior_from_top_of_library(
        &'static self,
        db: &Database,
        source: CardId,
        target: CardId,
        results: &mut PendingResults,
    ) {
        let _ = db;
        let _ = source;
        let _ = target;
        let _ = results;
        unreachable!()
    }

    fn push_behavior_with_targets(
        &'static self,
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        apply_to_self: bool,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    );

    fn replace_draw(
        &'static self,
        player: &mut Player,
        db: &mut Database,
        replacements: &mut IntoIter<ReplacementEffectId>,
        controller: Controller,
        count: usize,
        results: &mut PendingResults,
    ) {
        let _ = player;
        let _ = db;
        let _ = replacements;
        let _ = controller;
        let _ = count;
        let _ = results;
        unreachable!()
    }

    fn replace_token_creation(
        &'static self,
        db: &mut Database,
        source: CardId,
        replacements: &mut IntoIter<ReplacementEffectId>,
        token: CardId,
        modifiers: &[ModifyBattlefield],
        results: &mut PendingResults,
    ) {
        let _ = db;
        let _ = source;
        let _ = replacements;
        let _ = token;
        let _ = modifiers;
        let _ = results;
        unreachable!()
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
            protogen::effects::effect::Effect::BattlefieldModifier(value) => Ok(Self(Box::leak(
                Box::new(BattlefieldModifier::try_from(value)?),
            ))),
            protogen::effects::effect::Effect::ModifyTarget(value) => {
                Ok(Self(Box::leak(Box::new(ModifyTarget::try_from(value)?))))
            }
            protogen::effects::effect::Effect::Cascade(_) => Ok(Self(&Cascade)),
            protogen::effects::effect::Effect::ControllerDrawCards(value) => {
                Ok(Self(Box::leak(Box::new(ControllerDrawsCards {
                    count: usize::try_from(value.count)?,
                }))))
            }
            protogen::effects::effect::Effect::ControllerLosesLife(value) => {
                Ok(Self(Box::leak(Box::new(ControllerLosesLife {
                    count: usize::try_from(value.count)?,
                }))))
            }
            protogen::effects::effect::Effect::CopyOfAnyCreatureNonTargeting(_) => {
                Ok(Self(&CopyOfAnyCreatureNonTargeting))
            }
            protogen::effects::effect::Effect::CounterSpell(value) => {
                Ok(Self(Box::leak(Box::new(CounterSpell::try_from(value)?))))
            }
            protogen::effects::effect::Effect::CreateToken(value) => {
                Ok(Self(Box::leak(Box::new(CreateToken::try_from(value)?))))
            }
            protogen::effects::effect::Effect::CreateTokenCopy(value) => {
                Ok(Self(Box::leak(Box::new(CreateTokenCopy::try_from(value)?))))
            }
            protogen::effects::effect::Effect::DealDamage(value) => {
                Ok(Self(Box::leak(Box::new(DealDamage::try_from(value)?))))
            }
            protogen::effects::effect::Effect::DestroyEach(value) => {
                Ok(Self(Box::leak(Box::new(DestroyEach::try_from(value)?))))
            }
            protogen::effects::effect::Effect::DestroyTarget(value) => {
                Ok(Self(Box::leak(Box::new(DestroyTarget::try_from(value)?))))
            }
            protogen::effects::effect::Effect::Discover(value) => {
                Ok(Self(Box::leak(Box::new(Discover::try_from(value)?))))
            }
            protogen::effects::effect::Effect::Equip(value) => {
                Ok(Self(Box::leak(Box::new(Equip::try_from(value)?))))
            }
            protogen::effects::effect::Effect::ExileTarget(value) => {
                Ok(Self(Box::leak(Box::new(ExileTarget::try_from(value)?))))
            }
            protogen::effects::effect::Effect::ExileTargetCreatureManifestTopOfLibrary(_) => {
                Ok(Self(&ExileTargetCreatureManifestTopOfLibrary))
            }
            protogen::effects::effect::Effect::ExileTargetGraveyard(_) => {
                Ok(Self(&ExileTargetGraveyard))
            }
            protogen::effects::effect::Effect::ForEachManaOfSource(value) => Ok(Self(Box::leak(
                Box::new(ForEachManaOfSource::try_from(value)?),
            ))),
            protogen::effects::effect::Effect::TargetGainsCounters(value) => Ok(Self(Box::leak(
                Box::new(TargetGainsCounters::try_from(value)?),
            ))),
            protogen::effects::effect::Effect::GainLife(value) => {
                Ok(Self(Box::leak(Box::new(GainLife::try_from(value)?))))
            }
            protogen::effects::effect::Effect::Mill(value) => {
                Ok(Self(Box::leak(Box::new(Mill::try_from(value)?))))
            }
            protogen::effects::effect::Effect::Modal(value) => {
                Ok(Self(Box::leak(Box::new(Modal::try_from(value)?))))
            }
            protogen::effects::effect::Effect::MultiplyTokens(value) => {
                Ok(Self(Box::leak(Box::new(MultiplyTokens::try_from(value)?))))
            }
            protogen::effects::effect::Effect::PayCostThen(value) => {
                Ok(Self(Box::leak(Box::new(PayCostThen::try_from(value)?))))
            }
            protogen::effects::effect::Effect::ReturnFromGraveyardToBattlefield(value) => Ok(Self(
                Box::leak(Box::new(ReturnFromGraveyardToBattlefield::try_from(value)?)),
            )),
            protogen::effects::effect::Effect::ReturnFromGraveyardToHand(value) => Ok(Self(
                Box::leak(Box::new(ReturnFromGraveyardToHand::try_from(value)?)),
            )),
            protogen::effects::effect::Effect::ReturnFromGraveyardToLibrary(value) => Ok(Self(
                Box::leak(Box::new(ReturnFromGraveyardToLibrary::try_from(value)?)),
            )),
            protogen::effects::effect::Effect::ReturnTransformed(value) => Ok(Self(Box::leak(
                Box::new(ReturnTransformed::try_from(value)?),
            ))),
            protogen::effects::effect::Effect::ReturnSelfToHand(_) => Ok(Self(&ReturnSelfToHand)),
            protogen::effects::effect::Effect::ReturnTargetToHand(value) => Ok(Self(Box::leak(
                Box::new(ReturnTargetToHand::try_from(value)?),
            ))),
            protogen::effects::effect::Effect::RevealEachTopOfLibrary(value) => Ok(Self(
                Box::leak(Box::new(RevealEachTopOfLibrary::try_from(value)?)),
            )),
            protogen::effects::effect::Effect::Scry(value) => {
                Ok(Self(Box::leak(Box::new(Scry::try_from(value)?))))
            }
            protogen::effects::effect::Effect::SelfExplores(_) => Ok(Self(&SelfExplores)),
            protogen::effects::effect::Effect::TargetControllerGainsTokens(value) => Ok(Self(
                Box::leak(Box::new(TargetControllerGainsTokens::try_from(value)?)),
            )),
            protogen::effects::effect::Effect::TargetExplores(_) => {
                Ok(Self(&TargetCreatureExplores))
            }
            protogen::effects::effect::Effect::TargetToTopOfLibrary(value) => Ok(Self(Box::leak(
                Box::new(TargetToTopOfLibrary::try_from(value)?),
            ))),
            protogen::effects::effect::Effect::Transform(_) => Ok(Self(&Transform)),
            protogen::effects::effect::Effect::TutorLibrary(value) => {
                Ok(Self(Box::leak(Box::new(TutorLibrary::try_from(value)?))))
            }
            protogen::effects::effect::Effect::UntapThis(_) => Ok(Self(&UntapThis)),
            protogen::effects::effect::Effect::UntapTarget(_) => Ok(Self(&UntapTarget)),
        }
    }
}

#[derive(Debug, Deref, Clone, DerefMut, Component, Default)]
pub struct Effects(pub Vec<AnyEffect>);

#[derive(Debug, Clone)]
pub struct AnyEffect {
    pub effect: Effect,
    pub threshold: Option<Effect>,
    pub oracle_text: String,
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
            oracle_text: value.oracle_text.clone(),
        })
    }
}

impl AnyEffect {
    pub fn effect(&self, db: &mut Database, controller: Controller) -> Effect {
        if self.threshold.is_some()
            && Battlefield::number_of_cards_in_graveyard(db, controller) >= 7
        {
            self.threshold.as_ref().unwrap().clone()
        } else {
            self.effect.clone()
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

    pub fn needs_targets(&self, db: &mut Database, controller: Controller) -> usize {
        let effect = self.effect(db, controller);
        effect.needs_targets()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenCreature {
    pub name: String,
    pub types: IndexSet<Type>,
    pub subtypes: IndexSet<Subtype>,
    pub colors: HashSet<Color>,
    pub keywords: ::counter::Counter<Keyword>,
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
                .collect::<anyhow::Result<_>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<_>>()?,
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
    Map,
    Creature(Box<TokenCreature>),
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
                Ok(Self::Creature(Box::new(creature.try_into()?)))
            }
            protogen::effects::create_token::Token::Map(_) => Ok(Self::Map),
        }
    }
}

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
pub enum Replacing {
    Draw,
    Etb,
    TokenCreation,
}
}

impl From<&protogen::effects::replacement_effect::Replacing> for Replacing {
    fn from(value: &protogen::effects::replacement_effect::Replacing) -> Self {
        match value {
            protogen::effects::replacement_effect::Replacing::Draw(_) => Self::Draw,
            protogen::effects::replacement_effect::Replacing::Etb(_) => Self::Etb,
            protogen::effects::replacement_effect::Replacing::TokenCreation(_) => {
                Self::TokenCreation
            }
        }
    }
}

#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct ReplacementEffects(pub Vec<ReplacementEffectId>);

#[derive(Debug, Clone)]
pub struct ReplacementEffect {
    pub replacing: Replacing,
    pub controller: ControllerRestriction,
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
            controller: value.controller.get_or_default().try_into()?,
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

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub struct Modes(pub Vec<Mode>);

#[derive(Debug, Clone)]
pub struct Mode {
    pub effects: Vec<AnyEffect>,
}

impl TryFrom<&protogen::effects::Mode> for Mode {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Mode) -> Result<Self, Self::Error> {
        Ok(Self {
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}
