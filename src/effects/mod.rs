pub(crate) mod battle_cry;
pub(crate) mod battlefield_modifier;
pub(crate) mod cascade;
pub(crate) mod controller_draws_cards;
pub(crate) mod controller_loses_life;
pub(crate) mod copy_of_any_creature_non_targeting;
pub(crate) mod counter_spell;
pub(crate) mod create_token;
pub(crate) mod create_token_copy;
pub(crate) mod cycling;
pub(crate) mod deal_damage;
pub(crate) mod destroy_each;
pub(crate) mod destroy_target;
pub(crate) mod discover;
pub(crate) mod equip;
pub(crate) mod exile_target;
pub(crate) mod exile_target_creature_manifest_top_of_library;
pub(crate) mod exile_target_graveyard;
pub(crate) mod foreach_mana_of_source;
pub(crate) mod gain_life;
pub(crate) mod mill;
pub(crate) mod modal;
pub(crate) mod modify_target;
pub(crate) mod multiply_tokens;
pub(crate) mod pay_cost_then;
pub(crate) mod return_from_graveyard_to_battlefield;
pub(crate) mod return_from_graveyard_to_hand;
pub(crate) mod return_from_graveyard_to_library;
pub(crate) mod return_self_to_hand;
pub(crate) mod return_target_to_hand;
pub(crate) mod return_transformed;
pub(crate) mod reveal_each_top_of_library;
pub(crate) mod scry;
pub(crate) mod self_explores;
pub(crate) mod tap_target;
pub(crate) mod target_controller_gains_tokens;
pub(crate) mod target_creature_explores;
pub(crate) mod target_gains_counters;
pub(crate) mod target_to_top_of_library;
pub(crate) mod transform;
pub(crate) mod tutor_library;
pub(crate) mod untap_target;
pub(crate) mod untap_this;

use std::{collections::HashSet, fmt::Debug, str::FromStr, vec::IntoIter};

use anyhow::{anyhow, Context};
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    abilities::{ActivatedAbility, GainManaAbility, StaticAbility},
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
        cycling::Cycling,
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
        tap_target::TapTarget,
        target_controller_gains_tokens::TargetControllerGainsTokens,
        target_creature_explores::TargetCreatureExplores,
        target_gains_counters::{Counter, TargetGainsCounters},
        target_to_top_of_library::TargetToTopOfLibrary,
        transform::Transform,
        tutor_library::TutorLibrary,
        untap_target::UntapTarget,
        untap_this::UntapThis,
    },
    in_play::{self, CardId, Database, OnBattlefield, ReplacementEffectId},
    newtype_enum::newtype_enum,
    player::{AllPlayers, Controller, Player},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
    types::{Subtype, Type},
    Battlefield,
};

#[derive(Debug, Clone, Deref, DerefMut)]
pub(crate) struct Effect(pub(crate) &'static (dyn EffectBehaviors + Send + Sync));

pub(crate) use battlefield_modifier::BattlefieldModifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Destination {
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
pub(crate)enum EffectDuration {
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

#[derive(Debug, Clone)]
pub struct NumberOfPermanentsMatching {
    pub(crate) controller: ControllerRestriction,
    pub(crate) types: IndexSet<Type>,
    pub(crate) subtypes: IndexSet<Subtype>,
}

impl TryFrom<&protogen::effects::dynamic_power_toughness::NumberOfPermanentsMatching>
    for NumberOfPermanentsMatching
{
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::dynamic_power_toughness::NumberOfPermanentsMatching,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            controller: value.controller.get_or_default().try_into()?,
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<_>>()?,

            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl NumberOfPermanentsMatching {
    pub(crate) fn matching(&self, db: &mut Database, source: CardId) -> usize {
        in_play::cards::<OnBattlefield>(db)
            .into_iter()
            .filter(|card| {
                match self.controller {
                    ControllerRestriction::Any => {}
                    ControllerRestriction::You => {
                        if card.controller(db) != source.controller(db) {
                            return false;
                        }
                    }
                    ControllerRestriction::Opponent => {
                        if card.controller(db) == source.controller(db) {
                            return false;
                        }
                    }
                }

                if !card.types_intersect(db, &self.types) {
                    return false;
                }

                if !card.subtypes_intersect(db, &self.subtypes) {
                    return false;
                }

                true
            })
            .count()
    }
}

#[derive(Debug, Clone, Component)]
pub(crate) enum DynamicPowerToughness {
    NumberOfCountersOnThis(Counter),
    NumberOfPermanentsMatching(NumberOfPermanentsMatching),
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
            protogen::effects::dynamic_power_toughness::Source::NumberOfPermanentsMatching(
                value,
            ) => Ok(Self::NumberOfPermanentsMatching(value.try_into()?)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ModifyBattlefield {
    pub(crate) base_power: Option<i32>,
    pub(crate) base_toughness: Option<i32>,

    pub(crate) add_power: Option<i32>,
    pub(crate) add_toughness: Option<i32>,

    pub(crate) dynamic_power_toughness: Option<DynamicPowerToughness>,

    pub(crate) add_types: IndexSet<Type>,
    pub(crate) add_subtypes: IndexSet<Subtype>,

    pub(crate) remove_types: IndexSet<Type>,
    pub(crate) remove_subtypes: IndexSet<Subtype>,

    pub(crate) add_colors: HashSet<Color>,

    pub(crate) add_static_abilities: Vec<StaticAbility>,
    pub(crate) add_ability: Option<ActivatedAbility>,
    pub(crate) mana_ability: Option<GainManaAbility>,

    pub(crate) remove_all_subtypes: bool,
    pub(crate) remove_all_abilities: bool,
    pub(crate) remove_all_colors: bool,

    pub(crate) entire_battlefield: bool,
    pub(crate) global: bool,

    pub(crate) add_keywords: ::counter::Counter<Keyword>,
    pub(crate) remove_keywords: HashSet<Keyword>,
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
            add_static_abilities: value
                .add_static_abilities
                .iter()
                .map(StaticAbility::try_from)
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

pub(crate) trait EffectBehaviors: Debug {
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

    fn cycling(&'static self) -> bool {
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
            protogen::effects::effect::Effect::Cycling(value) => {
                Ok(Self(Box::leak(Box::new(Cycling::try_from(value)?))))
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
            protogen::effects::effect::Effect::TapTarget(value) => {
                Ok(Self(Box::leak(Box::new(TapTarget::try_from(value)?))))
            }
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
pub(crate) struct Effects(pub(crate) Vec<AnyEffect>);

#[derive(Debug, Clone)]
pub struct AnyEffect {
    pub(crate) effect: Effect,
    pub(crate) threshold: Option<Effect>,
    pub(crate) oracle_text: String,
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
    pub(crate) fn effect(&self, db: &mut Database, controller: Controller) -> Effect {
        if self.threshold.is_some()
            && Battlefield::number_of_cards_in_graveyard(db, controller) >= 7
        {
            self.threshold.as_ref().unwrap().clone()
        } else {
            self.effect.clone()
        }
    }

    pub(crate) fn into_effect(self, db: &mut Database, controller: Controller) -> Effect {
        if self.threshold.is_some()
            && Battlefield::number_of_cards_in_graveyard(db, controller) >= 7
        {
            self.threshold.unwrap()
        } else {
            self.effect
        }
    }

    pub(crate) fn needs_targets(&self, db: &mut Database, controller: Controller) -> usize {
        let effect = self.effect(db, controller);
        effect.needs_targets()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TokenCreature {
    pub(crate) name: String,
    pub(crate) types: IndexSet<Type>,
    pub(crate) subtypes: IndexSet<Subtype>,
    pub(crate) colors: HashSet<Color>,
    pub(crate) keywords: ::counter::Counter<Keyword>,
    pub(crate) dynamic_power_toughness: Option<DynamicPowerToughness>,
    pub(crate) power: usize,
    pub(crate) toughness: usize,
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
            dynamic_power_toughness: value
                .dynamic_power_toughness
                .as_ref()
                .map_or(Ok(None), |dynamic| dynamic.try_into().map(Some))?,
            power: usize::try_from(value.power)?,
            toughness: usize::try_from(value.toughness)?,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Token {
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
pub(crate)enum Replacing {
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
pub(crate) struct ReplacementEffects(pub(crate) Vec<ReplacementEffectId>);

#[derive(Debug, Clone)]
pub(crate) struct ReplacementEffect {
    pub(crate) replacing: Replacing,
    pub(crate) controller: ControllerRestriction,
    pub(crate) restrictions: Vec<Restriction>,
    pub(crate) effects: Vec<AnyEffect>,
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
pub(crate) struct Modes(pub(crate) Vec<Mode>);

#[derive(Debug, Clone)]
pub(crate) struct Mode {
    pub(crate) effects: Vec<AnyEffect>,
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
