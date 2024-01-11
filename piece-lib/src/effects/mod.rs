pub(crate) mod apply_then_if_was;
pub(crate) mod battle_cry;
pub(crate) mod battlefield_modifier;
pub(crate) mod cant_attack_this_turn;
pub(crate) mod cascade;
pub(crate) mod controller_discards;
pub(crate) mod controller_draws_cards;
pub(crate) mod controller_loses_life;
pub(crate) mod copy_of_any_creature_non_targeting;
pub(crate) mod copy_spell_or_ability;
pub(crate) mod counter_spell;
pub(crate) mod counter_spell_unless_pay;
pub(crate) mod create_token;
pub(crate) mod create_token_copy;
pub(crate) mod cycling;
pub(crate) mod deal_damage;
pub(crate) mod destroy_each;
pub(crate) mod destroy_target;
pub(crate) mod discover;
pub(crate) mod equip;
pub(crate) mod examine_top_cards;
pub(crate) mod exile_target;
pub(crate) mod exile_target_creature_manifest_top_of_library;
pub(crate) mod exile_target_graveyard;
pub(crate) mod for_each_player_choose_then;
pub(crate) mod foreach_mana_of_source;
pub(crate) mod gain_life;
pub(crate) mod if_then_else;
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
pub(crate) mod tap_this;
pub(crate) mod target_controller_gains_tokens;
pub(crate) mod target_creature_explores;
pub(crate) mod target_gains_counters;
pub(crate) mod target_to_top_of_library;
pub(crate) mod transform;
pub(crate) mod tutor_library;
pub(crate) mod untap_target;
pub(crate) mod untap_this;

use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    vec::IntoIter,
};

use anyhow::anyhow;
use derive_more::{Deref, DerefMut};
use enum_dispatch::enum_dispatch;
use itertools::Itertools;

use crate::{
    abilities::{ActivatedAbility, GainManaAbility, StaticAbility},
    card::replace_symbols,
    effects::{
        apply_then_if_was::ApplyThenIfWas, battle_cry::BattleCry,
        cant_attack_this_turn::CantAttackThisTurn, cascade::Cascade,
        controller_discards::ControllerDiscards, controller_draws_cards::ControllerDrawsCards,
        controller_loses_life::ControllerLosesLife,
        copy_of_any_creature_non_targeting::CopyOfAnyCreatureNonTargeting,
        copy_spell_or_ability::CopySpellOrAbility, counter_spell::CounterSpellOrAbility,
        counter_spell_unless_pay::CounterSpellUnlessPay, create_token::CreateToken,
        create_token_copy::CreateTokenCopy, cycling::Cycling, deal_damage::DealDamage,
        destroy_each::DestroyEach, destroy_target::DestroyTarget, discover::Discover, equip::Equip,
        examine_top_cards::ExamineTopCards, exile_target::ExileTarget,
        exile_target_creature_manifest_top_of_library::ExileTargetCreatureManifestTopOfLibrary,
        exile_target_graveyard::ExileTargetGraveyard,
        for_each_player_choose_then::ForEachPlayerChooseThen,
        foreach_mana_of_source::ForEachManaOfSource, gain_life::GainLife, if_then_else::IfThenElse,
        mill::Mill, modal::Modal, modify_target::ModifyTarget, multiply_tokens::MultiplyTokens,
        pay_cost_then::PayCostThen,
        return_from_graveyard_to_battlefield::ReturnFromGraveyardToBattlefield,
        return_from_graveyard_to_hand::ReturnFromGraveyardToHand,
        return_from_graveyard_to_library::ReturnFromGraveyardToLibrary,
        return_self_to_hand::ReturnSelfToHand, return_target_to_hand::ReturnTargetToHand,
        return_transformed::ReturnTransformed, reveal_each_top_of_library::RevealEachTopOfLibrary,
        scry::Scry, self_explores::SelfExplores, tap_target::TapTarget, tap_this::TapThis,
        target_controller_gains_tokens::TargetControllerGainsTokens,
        target_creature_explores::TargetCreatureExplores,
        target_gains_counters::TargetGainsCounters, target_to_top_of_library::TargetToTopOfLibrary,
        transform::Transform, tutor_library::TutorLibrary, untap_target::UntapTarget,
        untap_this::UntapThis,
    },
    in_play::{CardId, Database},
    log::LogId,
    pending_results::PendingResults,
    player::{Controller, Owner},
    protogen::targets::Restriction,
    protogen::{
        self,
        color::Color,
        counters::Counter,
        empty::Empty,
        types::{Subtype, Type},
    },
    stack::ActiveTarget,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[enum_dispatch(EffectBehaviors)]
pub(crate) enum Effect {
    ApplyThenIfWas(ApplyThenIfWas),
    BattleCry(BattleCry),
    BattlefieldModifier(BattlefieldModifier),
    CantAttackThisTurn(CantAttackThisTurn),
    Cascade(Cascade),
    ControllerDiscards(ControllerDiscards),
    ControllerDrawsCards(ControllerDrawsCards),
    ControllerLosesLife(ControllerLosesLife),
    CopyOfAnyCreatureNonTargeting(CopyOfAnyCreatureNonTargeting),
    CopySpellOrAbility(CopySpellOrAbility),
    CounterSpellOrAbility(CounterSpellOrAbility),
    CounterSpellUnlessPay(CounterSpellUnlessPay),
    CreateToken(CreateToken),
    CreateTokenCopy(CreateTokenCopy),
    Cycling(Cycling),
    DealDamage(DealDamage),
    DestroyEach(DestroyEach),
    DestroyTarget(DestroyTarget),
    Discover(Discover),
    Equip(Equip),
    ExamineTopCards(ExamineTopCards),
    ExileTarget(ExileTarget),
    ExileTargetCreatureManifestTopOfLibrary(ExileTargetCreatureManifestTopOfLibrary),
    ExileTargetGraveyard(ExileTargetGraveyard),
    ForEachPlayerChooseThen(ForEachPlayerChooseThen),
    ForEachManaOfSource(ForEachManaOfSource),
    GainLife(GainLife),
    IfThenElse(IfThenElse),
    Mill(Mill),
    Modal(Modal),
    ModifyTarget(ModifyTarget),
    MultiplyTokens(MultiplyTokens),
    PayCostThen(PayCostThen),
    ReturnFromGraveyardToBattlefield(ReturnFromGraveyardToBattlefield),
    ReturnFromGraveyardToHand(ReturnFromGraveyardToHand),
    ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary),
    ReturnSelfToHand(ReturnSelfToHand),
    ReturnTargetToHand(ReturnTargetToHand),
    ReturnTransformed(ReturnTransformed),
    RevealEachTopOfLibrary(RevealEachTopOfLibrary),
    Scry(Scry),
    SelfExplores(SelfExplores),
    TapTarget(TapTarget),
    TapThis(TapThis),
    TargetControllerGainsTokens(TargetControllerGainsTokens),
    TargetCreatureExplores(TargetCreatureExplores),
    TargetGainsCounters(TargetGainsCounters),
    TargetToTopOfLibrary(TargetToTopOfLibrary),
    Transform(Transform),
    TutorLibrary(TutorLibrary),
    UntapTarget(UntapTarget),
    UntapThis(UntapThis),
}

pub(crate) use battlefield_modifier::BattlefieldModifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::AsRefStr)]
pub(crate) enum Destination {
    Hand,
    TopOfLibrary,
    BottomOfLibrary,
    Graveyard,
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
            protogen::effects::destination::Destination::BottomOfLibrary(_) => {
                Self::BottomOfLibrary
            }
            protogen::effects::destination::Destination::Graveyard(_) => Self::Graveyard,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NumberOfPermanentsMatching {
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::NumberOfPermanentsMatching> for NumberOfPermanentsMatching {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::NumberOfPermanentsMatching,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value.restrictions.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DynamicPowerToughness {
    NumberOfCountersOnThis(protobuf::EnumOrUnknown<Counter>),
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
                Ok(Self::NumberOfCountersOnThis(counter.counter))
            }
            protogen::effects::dynamic_power_toughness::Source::NumberOfPermanentsMatching(
                value,
            ) => Ok(Self::NumberOfPermanentsMatching(value.try_into()?)),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModifyBattlefield {
    pub(crate) base_power: Option<i32>,
    pub(crate) base_toughness: Option<i32>,

    pub(crate) add_power: Option<i32>,
    pub(crate) add_toughness: Option<i32>,

    pub(crate) dynamic_power_toughness: Option<DynamicPowerToughness>,

    pub(crate) add_types: HashMap<i32, Empty>,
    pub(crate) add_subtypes: HashMap<i32, Empty>,

    pub(crate) remove_types: HashMap<i32, Empty>,
    pub(crate) remove_subtypes: HashMap<i32, Empty>,

    pub(crate) add_colors: Vec<protobuf::EnumOrUnknown<Color>>,

    pub(crate) add_static_abilities: Vec<StaticAbility>,
    pub(crate) add_ability: Option<ActivatedAbility>,
    pub(crate) mana_ability: Option<GainManaAbility>,

    pub(crate) remove_all_types: bool,
    pub(crate) remove_all_creature_types: bool,
    pub(crate) remove_all_abilities: bool,
    pub(crate) remove_all_colors: bool,

    pub(crate) entire_battlefield: bool,
    pub(crate) global: bool,

    pub(crate) add_keywords: HashMap<i32, u32>,
    pub(crate) remove_keywords: HashMap<i32, u32>,
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
            add_types: value.add_types.clone(),
            add_subtypes: value.add_subtypes.clone(),
            add_colors: value.add_colors.clone(),
            remove_types: value.remove_types.clone(),
            remove_subtypes: value.remove_subtypes.clone(),
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
            remove_all_types: value.remove_all_types,
            remove_all_creature_types: value.remove_all_creature_types,
            remove_all_abilities: value.remove_all_abilities,
            remove_all_colors: value.remove_all_colors,
            entire_battlefield: value.entire_battlefield,
            global: value.global,
            add_keywords: value.add_keywords.clone(),
            remove_keywords: value.remove_keywords.clone(),
        })
    }
}

#[enum_dispatch]
pub(crate) trait EffectBehaviors: Debug {
    fn choices(&self, db: &Database, targets: &[ActiveTarget]) -> Vec<String> {
        targets
            .iter()
            .map(|target| target.display(db))
            .collect_vec()
    }

    fn modes(&self) -> Vec<Mode> {
        vec![]
    }

    fn is_sorcery_speed(&self) -> bool {
        false
    }

    fn is_equip(&self) -> bool {
        false
    }

    fn cycling(&self) -> bool {
        false
    }

    fn needs_targets(&self, db: &Database, source: CardId) -> usize;

    fn wants_targets(&self, db: &Database, source: CardId) -> usize;

    fn valid_targets(
        &self,
        db: &Database,
        source: CardId,
        log_session: LogId,
        controller: Controller,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        let _ = db;
        let _ = source;
        let _ = log_session;
        let _ = controller;
        let _ = already_chosen;
        vec![]
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    );

    fn push_behavior_from_top_of_library(
        &self,
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
        &self,
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        apply_to_self: bool,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    );

    fn replace_draw(
        &self,
        db: &mut Database,
        player: Owner,
        replacements: &mut IntoIter<(CardId, ReplacementAbility)>,
        controller: Controller,
        count: usize,
        results: &mut PendingResults,
    ) {
        let _ = db;
        let _ = player;
        let _ = replacements;
        let _ = controller;
        let _ = count;
        let _ = results;
        unreachable!()
    }

    fn replace_token_creation(
        &self,
        db: &mut Database,
        source: CardId,
        replacements: &mut IntoIter<(CardId, ReplacementAbility)>,
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
            protogen::effects::effect::Effect::ApplyThenIfWas(value) => {
                Ok(Self::from(ApplyThenIfWas::try_from(value)?))
            }
            protogen::effects::effect::Effect::BattlefieldModifier(value) => {
                Ok(Self::from(BattlefieldModifier::try_from(value)?))
            }
            protogen::effects::effect::Effect::ModifyTarget(value) => {
                Ok(Self::from(ModifyTarget::try_from(value)?))
            }
            protogen::effects::effect::Effect::CantAttackThisTurn(value) => {
                Ok(Self::from(CantAttackThisTurn::try_from(value)?))
            }
            protogen::effects::effect::Effect::Cascade(_) => Ok(Self::from(Cascade)),
            protogen::effects::effect::Effect::ControllerDiscards(value) => {
                Ok(Self::from(ControllerDiscards::try_from(value)?))
            }
            protogen::effects::effect::Effect::ControllerDrawCards(value) => {
                Ok(Self::from(ControllerDrawsCards::try_from(value)?))
            }
            protogen::effects::effect::Effect::ControllerLosesLife(value) => {
                Ok(Self::from(ControllerLosesLife::try_from(value)?))
            }
            protogen::effects::effect::Effect::CopyOfAnyCreatureNonTargeting(_) => {
                Ok(Self::from(CopyOfAnyCreatureNonTargeting))
            }
            protogen::effects::effect::Effect::CopySpellOrAbility(value) => {
                Ok(Self::from(CopySpellOrAbility::try_from(value)?))
            }
            protogen::effects::effect::Effect::CounterSpell(value) => {
                Ok(Self::from(CounterSpellOrAbility::try_from(value)?))
            }
            protogen::effects::effect::Effect::CounterSpellUnlessPay(value) => {
                Ok(Self::from(CounterSpellUnlessPay::try_from(value)?))
            }
            protogen::effects::effect::Effect::CreateToken(value) => {
                Ok(Self::from(CreateToken::try_from(value)?))
            }
            protogen::effects::effect::Effect::CreateTokenCopy(value) => {
                Ok(Self::from(CreateTokenCopy::try_from(value)?))
            }
            protogen::effects::effect::Effect::Cycling(value) => {
                Ok(Self::from(Cycling::try_from(value)?))
            }
            protogen::effects::effect::Effect::DealDamage(value) => {
                Ok(Self::from(DealDamage::try_from(value)?))
            }
            protogen::effects::effect::Effect::DestroyEach(value) => {
                Ok(Self::from(DestroyEach::try_from(value)?))
            }
            protogen::effects::effect::Effect::DestroyTarget(value) => {
                Ok(Self::from(DestroyTarget::try_from(value)?))
            }
            protogen::effects::effect::Effect::Discover(value) => {
                Ok(Self::from(Discover::try_from(value)?))
            }
            protogen::effects::effect::Effect::Equip(value) => {
                Ok(Self::from(Equip::try_from(value)?))
            }
            protogen::effects::effect::Effect::ExamineTopCards(value) => {
                Ok(Self::from(ExamineTopCards::try_from(value)?))
            }
            protogen::effects::effect::Effect::ExileTarget(value) => {
                Ok(Self::from(ExileTarget::try_from(value)?))
            }
            protogen::effects::effect::Effect::ExileTargetCreatureManifestTopOfLibrary(_) => {
                Ok(Self::from(ExileTargetCreatureManifestTopOfLibrary))
            }
            protogen::effects::effect::Effect::ExileTargetGraveyard(_) => {
                Ok(Self::from(ExileTargetGraveyard))
            }
            protogen::effects::effect::Effect::ForEachManaOfSource(value) => {
                Ok(Self::from(ForEachManaOfSource::try_from(value)?))
            }
            protogen::effects::effect::Effect::ForEachPlayerChooseThen(value) => {
                Ok(Self::from(ForEachPlayerChooseThen::try_from(value)?))
            }
            protogen::effects::effect::Effect::TargetGainsCounters(value) => {
                Ok(Self::from(TargetGainsCounters::try_from(value)?))
            }
            protogen::effects::effect::Effect::GainLife(value) => {
                Ok(Self::from(GainLife::try_from(value)?))
            }
            protogen::effects::effect::Effect::IfThenElse(value) => {
                Ok(Self::from(IfThenElse::try_from(value)?))
            }
            protogen::effects::effect::Effect::Mill(value) => {
                Ok(Self::from(Mill::try_from(value)?))
            }
            protogen::effects::effect::Effect::Modal(value) => {
                Ok(Self::from(Modal::try_from(value)?))
            }
            protogen::effects::effect::Effect::MultiplyTokens(value) => {
                Ok(Self::from(MultiplyTokens::try_from(value)?))
            }
            protogen::effects::effect::Effect::PayCostThen(value) => {
                Ok(Self::from(PayCostThen::try_from(value)?))
            }
            protogen::effects::effect::Effect::ReturnFromGraveyardToBattlefield(value) => Ok(
                Self::from(ReturnFromGraveyardToBattlefield::try_from(value)?),
            ),
            protogen::effects::effect::Effect::ReturnFromGraveyardToHand(value) => {
                Ok(Self::from(ReturnFromGraveyardToHand::try_from(value)?))
            }
            protogen::effects::effect::Effect::ReturnFromGraveyardToLibrary(value) => {
                Ok(Self::from(ReturnFromGraveyardToLibrary::try_from(value)?))
            }
            protogen::effects::effect::Effect::ReturnTransformed(value) => {
                Ok(Self::from(ReturnTransformed::try_from(value)?))
            }
            protogen::effects::effect::Effect::ReturnSelfToHand(_) => {
                Ok(Self::from(ReturnSelfToHand))
            }
            protogen::effects::effect::Effect::ReturnTargetToHand(value) => {
                Ok(Self::from(ReturnTargetToHand::try_from(value)?))
            }
            protogen::effects::effect::Effect::RevealEachTopOfLibrary(value) => {
                Ok(Self::from(RevealEachTopOfLibrary::try_from(value)?))
            }
            protogen::effects::effect::Effect::Scry(value) => {
                Ok(Self::from(Scry::try_from(value)?))
            }
            protogen::effects::effect::Effect::SelfExplores(_) => Ok(Self::from(SelfExplores)),
            protogen::effects::effect::Effect::TapTarget(value) => {
                Ok(Self::from(TapTarget::try_from(value)?))
            }
            protogen::effects::effect::Effect::TapThis(_) => Ok(Self::from(TapThis)),
            protogen::effects::effect::Effect::TargetControllerGainsTokens(value) => {
                Ok(Self::from(TargetControllerGainsTokens::try_from(value)?))
            }
            protogen::effects::effect::Effect::TargetExplores(_) => {
                Ok(Self::from(TargetCreatureExplores))
            }
            protogen::effects::effect::Effect::TargetToTopOfLibrary(value) => {
                Ok(Self::from(TargetToTopOfLibrary::try_from(value)?))
            }
            protogen::effects::effect::Effect::Transform(_) => Ok(Self::from(Transform)),
            protogen::effects::effect::Effect::TutorLibrary(value) => {
                Ok(Self::from(TutorLibrary::try_from(value)?))
            }
            protogen::effects::effect::Effect::UntapThis(_) => Ok(Self::from(UntapThis)),
            protogen::effects::effect::Effect::UntapTarget(_) => Ok(Self::from(UntapTarget)),
        }
    }
}

#[derive(Debug, Deref, Clone, DerefMut, Default)]
pub(crate) struct Effects(pub(crate) Vec<AnyEffect>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnyEffect {
    pub(crate) effect: Effect,
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
            oracle_text: replace_symbols(&value.oracle_text),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenCreature {
    pub(crate) name: String,
    pub(crate) types: Vec<protobuf::EnumOrUnknown<Type>>,
    pub(crate) subtypes: Vec<protobuf::EnumOrUnknown<Subtype>>,
    pub(crate) colors: Vec<protobuf::EnumOrUnknown<Color>>,
    pub(crate) keywords: HashMap<i32, u32>,
    pub(crate) dynamic_power_toughness: Option<DynamicPowerToughness>,
    pub(crate) power: usize,
    pub(crate) toughness: usize,
}

impl TryFrom<&protogen::effects::create_token::Creature> for TokenCreature {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::create_token::Creature) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.clone(),
            types: value.typeline.types.clone(),
            subtypes: value.typeline.subtypes.clone(),
            colors: value.colors.clone(),
            keywords: value.keywords.clone(),
            dynamic_power_toughness: value
                .dynamic_power_toughness
                .as_ref()
                .map_or(Ok(None), |dynamic| dynamic.try_into().map(Some))?,
            power: usize::try_from(value.power)?,
            toughness: usize::try_from(value.toughness)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Replacing {
    Draw,
    Etb,
    TokenCreation,
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

#[derive(Debug, Clone)]
pub(crate) struct ReplacementAbility {
    pub(crate) replacing: Replacing,
    pub(crate) restrictions: Vec<Restriction>,
    pub(crate) effects: Vec<AnyEffect>,
}

impl TryFrom<&protogen::effects::ReplacementEffect> for ReplacementAbility {
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
            restrictions: value.restrictions.clone(),
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub(crate) struct Modes(pub(crate) Vec<Mode>);

#[derive(Debug, Clone, PartialEq, Eq)]
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
