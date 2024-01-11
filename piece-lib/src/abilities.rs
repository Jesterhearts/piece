use std::collections::{HashMap, HashSet};

use anyhow::anyhow;
use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    card::replace_symbols,
    cost::{AbilityCost, AbilityRestriction, AdditionalCost},
    counters::Counter,
    effects::{AnyEffect, BattlefieldModifier, EffectBehaviors},
    in_play::{ActivatedAbilityId, CardId, Database, GainManaAbilityId},
    log::LogId,
    pending_results::PendingResults,
    player::{mana_pool::SpendReason, Owner},
    protogen::{
        self,
        mana::ManaSource,
        mana::{Mana, ManaRestriction},
    },
    targets::Restriction,
    triggers::Trigger,
    turns::Phase,
};

#[derive(Debug, Clone)]
pub(crate) struct Enchant {
    pub(crate) modifiers: Vec<BattlefieldModifier>,
}

impl TryFrom<&protogen::abilities::Enchant> for Enchant {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::Enchant) -> Result<Self, Self::Error> {
        Ok(Self {
            modifiers: value
                .modifiers
                .iter()
                .map(BattlefieldModifier::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddKeywordsIf {
    pub(crate) keywords: HashMap<String, u32>,
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::static_ability::AddKeywordsIf> for AddKeywordsIf {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::static_ability::AddKeywordsIf,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            keywords: value.keywords.clone(),
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForceEtbTapped {
    pub(crate) restrictions: Vec<Restriction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StaticAbility {
    AddKeywordsIf(AddKeywordsIf),
    AllAbilitiesOfExiledWith {
        ability_restriction: Vec<AbilityRestriction>,
    },
    BattlefieldModifier(Box<BattlefieldModifier>),
    CantCastIfAttacked,
    ExtraLandsPerTurn(usize),
    ForceEtbTapped(ForceEtbTapped),
    GreenCannotBeCountered {
        restrictions: Vec<Restriction>,
    },
    PreventAttacks,
    PreventBlocks,
    PreventAbilityActivation,
    UntapEachUntapStep,
}

impl TryFrom<&protogen::effects::StaticAbility> for StaticAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::StaticAbility) -> Result<Self, Self::Error> {
        value
            .ability
            .as_ref()
            .ok_or_else(|| anyhow!("Expected ability to have an ability specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::effects::static_ability::Ability> for StaticAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::static_ability::Ability) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::static_ability::Ability::AddKeywordsIf(value) => {
                Ok(Self::AddKeywordsIf(value.try_into()?))
            }
            protogen::effects::static_ability::Ability::AllAbilitiesOfExiledWith(value) => {
                Ok(Self::AllAbilitiesOfExiledWith {
                    ability_restriction: value
                        .activation_restrictions
                        .iter()
                        .map(AbilityRestriction::try_from)
                        .collect::<anyhow::Result<_>>()?,
                })
            }
            protogen::effects::static_ability::Ability::CantCastIfAttacked(_) => {
                Ok(Self::CantCastIfAttacked)
            }
            protogen::effects::static_ability::Ability::BattlefieldModifier(modifier) => {
                Ok(Self::BattlefieldModifier(Box::new(modifier.try_into()?)))
            }
            protogen::effects::static_ability::Ability::ExtraLandsPerTurn(extra_lands) => {
                Ok(Self::ExtraLandsPerTurn(usize::try_from(extra_lands.count)?))
            }
            protogen::effects::static_ability::Ability::ForceEtbTapped(force) => {
                Ok(Self::ForceEtbTapped(ForceEtbTapped {
                    restrictions: force
                        .restrictions
                        .iter()
                        .map(Restriction::try_from)
                        .collect::<anyhow::Result<_>>()?,
                }))
            }
            protogen::effects::static_ability::Ability::GreenCannotBeCountered(ability) => {
                Ok(Self::GreenCannotBeCountered {
                    restrictions: ability
                        .restrictions
                        .iter()
                        .map(Restriction::try_from)
                        .collect::<anyhow::Result<_>>()?,
                })
            }
            protogen::effects::static_ability::Ability::PreventAttacks(_) => {
                Ok(Self::PreventAttacks)
            }
            protogen::effects::static_ability::Ability::PreventBlocks(_) => Ok(Self::PreventBlocks),
            protogen::effects::static_ability::Ability::PreventAbilityActivation(_) => {
                Ok(Self::PreventAbilityActivation)
            }
            protogen::effects::static_ability::Ability::UntapEachUntapStep(_) => {
                Ok(Self::UntapEachUntapStep)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivatedAbility {
    pub(crate) cost: AbilityCost,
    pub(crate) effects: Vec<AnyEffect>,
    pub(crate) apply_to_self: bool,
    pub(crate) oracle_text: String,
    pub(crate) sorcery_speed: bool,
    pub(crate) craft: bool,
}

impl TryFrom<&protogen::effects::ActivatedAbility> for ActivatedAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ActivatedAbility) -> Result<Self, Self::Error> {
        Ok(Self {
            cost: value
                .cost
                .as_ref()
                .ok_or_else(|| anyhow!("Expected ability to have a cost"))
                .and_then(AbilityCost::try_from)?,
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            apply_to_self: value.apply_to_self,
            oracle_text: replace_symbols(&value.oracle_text),
            sorcery_speed: value.sorcery_speed,
            craft: value.craft,
        })
    }
}

impl ActivatedAbility {
    pub(crate) fn can_be_played_from_hand(&self) -> bool {
        self.effects.iter().any(|effect| effect.effect.cycling())
    }

    pub(crate) fn can_be_activated(
        &self,
        db: &Database,
        source: CardId,
        id: &Ability,
        activator: crate::player::Owner,
        pending: &Option<PendingResults>,
    ) -> bool {
        let banned = db[source].modified_static_abilities.iter().any(|ability| {
            matches!(
                db[*ability].ability,
                StaticAbility::PreventAbilityActivation
            )
        });

        if banned {
            return false;
        }

        let in_battlefield = db.battlefield[db[source].controller].contains(&source);

        if pending.is_some() && !pending.as_ref().unwrap().is_empty() {
            return false;
        }

        let in_hand = db.hand[activator].contains(&source);
        if in_hand && !self.can_be_played_from_hand() {
            return false;
        }

        if self.can_be_played_from_hand() && !in_hand {
            return false;
        }

        if !in_battlefield {
            return false;
        }

        // TODO: Effects like Xantcha
        let controller = db[source].controller;
        if controller != activator {
            return false;
        }

        let is_sorcery = self.sorcery_speed
            || self
                .effects
                .iter()
                .any(|effect| effect.effect.is_sorcery_speed());
        if is_sorcery {
            if controller != db.turn.active_player() {
                return false;
            }

            if !matches!(
                db.turn.phase,
                Phase::PreCombatMainPhase | Phase::PostCombatMainPhase
            ) {
                return false;
            }

            if !db.stack.is_empty() {
                return false;
            }
        }

        if !can_pay_costs(db, id, &self.cost, source) {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone)]
pub struct TriggeredAbility {
    pub(crate) trigger: Trigger,
    pub(crate) effects: Vec<AnyEffect>,
    pub oracle_text: String,
}

impl TryFrom<&protogen::abilities::TriggeredAbility> for TriggeredAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::TriggeredAbility) -> Result<Self, Self::Error> {
        Ok(Self {
            trigger: value.trigger.get_or_default().try_into()?,
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            oracle_text: replace_symbols(&value.oracle_text),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum GainMana {
    Specific {
        gains: Vec<protobuf::EnumOrUnknown<Mana>>,
    },
    Choice {
        choices: Vec<protogen::effects::gain_mana::GainMana>,
    },
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
                gains: specific.gain.clone(),
            }),
            protogen::effects::gain_mana::Gain::Choice(choice) => Ok(Self::Choice {
                choices: choice.choices.clone(),
            }),
        }
    }
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub(crate) struct GainManaAbilities(pub(crate) Vec<GainManaAbility>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainManaAbility {
    pub(crate) cost: AbilityCost,
    pub(crate) gain: GainMana,
    pub(crate) mana_source: protobuf::EnumOrUnknown<ManaSource>,
    pub(crate) mana_restriction: protobuf::EnumOrUnknown<ManaRestriction>,
    pub(crate) oracle_text: String,
}

impl GainManaAbility {
    fn can_be_activated(
        &self,
        db: &Database,
        id: &Ability,
        source: CardId,
        activator: Owner,
    ) -> bool {
        if !db.battlefield[db[source].controller].contains(&source) {
            return false;
        }

        if db[source].controller != activator {
            return false;
        }

        can_pay_costs(db, id, &self.cost, source)
    }
}

impl TryFrom<&protogen::effects::GainManaAbility> for GainManaAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::GainManaAbility) -> Result<Self, Self::Error> {
        Ok(Self {
            cost: value.cost.get_or_default().try_into()?,
            gain: value.gain_mana.get_or_default().try_into()?,
            mana_source: value.mana_source,
            mana_restriction: value.mana_restriction,
            oracle_text: replace_symbols(&value.oracle_text),
        })
    }
}

#[derive(Debug, Clone)]
pub enum Ability {
    Activated(ActivatedAbilityId),
    Mana(GainManaAbilityId),
    EtbOrTriggered(Vec<AnyEffect>),
}

impl Ability {
    pub(crate) fn cost<'db>(&self, db: &'db Database) -> Option<&'db AbilityCost> {
        match self {
            Ability::Activated(id) => Some(&db[*id].ability.cost),
            Ability::Mana(id) => Some(&db[*id].ability.cost),
            Ability::EtbOrTriggered(_) => None,
        }
    }

    pub(crate) fn apply_to_self(&self, db: &Database) -> bool {
        match self {
            Ability::Activated(id) => db[*id].ability.apply_to_self,
            Ability::Mana(_) => false,
            Ability::EtbOrTriggered(_) => false,
        }
    }

    pub(crate) fn effects(&self, db: &Database) -> Vec<AnyEffect> {
        match self {
            Ability::Activated(id) => db[*id].ability.effects.clone(),
            Ability::Mana(_) => vec![],
            Ability::EtbOrTriggered(effects) => effects.clone(),
        }
    }

    pub fn text(&self, db: &Database) -> String {
        match self {
            Ability::Activated(id) => db[*id].ability.oracle_text.clone(),
            Ability::Mana(id) => db[*id].ability.oracle_text.clone(),
            Ability::EtbOrTriggered(effects) => {
                effects.iter().map(|effect| &effect.oracle_text).join("\n")
            }
        }
    }

    pub fn can_be_activated(
        &self,
        db: &Database,
        source: CardId,
        activator: crate::player::Owner,
        pending: &Option<PendingResults>,
    ) -> bool {
        match self {
            Ability::Activated(activated) => {
                if !db[*activated]
                    .ability
                    .can_be_activated(db, source, self, activator, pending)
                {
                    return false;
                }

                let targets = source.targets_for_ability(db, self, &HashSet::default());

                db[*activated]
                    .ability
                    .effects
                    .iter()
                    .map(|effect| effect.effect.needs_targets(db, source))
                    .zip(targets)
                    .all(|(needs, has)| has.len() >= needs)
            }
            Ability::Mana(id) => db[*id]
                .ability
                .can_be_activated(db, self, source, activator),
            Ability::EtbOrTriggered(_) => false,
        }
    }
}

pub(crate) fn can_pay_costs(
    db: &Database,
    id: &Ability,
    cost: &AbilityCost,
    source: CardId,
) -> bool {
    if cost.tap && db[source].tapped {
        return false;
    }
    let controller = db[source].controller;

    for cost in cost.additional_cost.iter() {
        match cost {
            AdditionalCost::SacrificeSource => {
                if !source.can_be_sacrificed(db) {
                    return false;
                }
            }
            AdditionalCost::PayLife(life) => {
                if db.all_players[controller].life_total <= life.count as i32 {
                    return false;
                }
            }
            AdditionalCost::SacrificePermanent(restrictions) => {
                let any_target = db.battlefield[controller].iter().any(|card| {
                    card.passes_restrictions(db, LogId::current(db), source, restrictions)
                });
                if !any_target {
                    return false;
                }
            }
            AdditionalCost::TapPermanent(restrictions) => {
                let any_target = db.battlefield[controller].iter().any(|card| {
                    card.passes_restrictions(db, LogId::current(db), source, restrictions)
                });
                if !any_target {
                    return false;
                }
            }
            AdditionalCost::ExileCard { restrictions } => {
                let any_target = db.battlefield[controller].iter().any(|card| {
                    card.passes_restrictions(db, LogId::current(db), source, restrictions)
                });
                if !any_target {
                    return false;
                }
            }
            AdditionalCost::ExileXOrMoreCards {
                minimum,
                restrictions,
            } => {
                let targets = db.battlefield[controller]
                    .iter()
                    .filter(|card| {
                        card.passes_restrictions(db, LogId::current(db), source, restrictions)
                    })
                    .count();

                if targets < *minimum {
                    return false;
                }
            }
            AdditionalCost::DiscardThis => {
                if !db.hand[controller].contains(&source) {
                    return false;
                }
            }
            AdditionalCost::RemoveCounter { counter, count } => {
                let counters = if let Counter::Any = counter {
                    db[source].counters.values().sum::<usize>()
                } else {
                    db[source]
                        .counters
                        .get(counter)
                        .copied()
                        .unwrap_or_default()
                };

                if counters < *count {
                    return false;
                }
            }

            // These are too complicated to compute, so just give up. The user can cancel if they can't actually pay.
            AdditionalCost::ExileCardsCmcX(_) => {}
            AdditionalCost::ExileSharingCardType { .. } => {}
            AdditionalCost::TapPermanentsPowerXOrMore { .. } => {}
        }
    }

    for restriction in cost.restrictions.iter() {
        match restriction {
            AbilityRestriction::AttackedWithXOrMoreCreatures(x) => {
                if db.turn.number_of_attackers_this_turn < *x {
                    return false;
                }
            }
            AbilityRestriction::OncePerTurn => match id {
                Ability::Activated(id) => {
                    if db.turn.activated_abilities.contains(id) {
                        return false;
                    }
                }
                Ability::Mana(_) => todo!(),
                Ability::EtbOrTriggered(_) => todo!(),
            },
        }
    }

    if !db.all_players[controller].can_meet_cost(
        db,
        &cost.mana_cost,
        &[],
        SpendReason::Activating(source),
    ) {
        return false;
    }

    true
}
