use std::str::FromStr;

use anyhow::{anyhow, Context};
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};

use crate::{
    card::{replace_symbols, Keyword},
    cost::AbilityCost,
    effects::{AnyEffect, BattlefieldModifier},
    in_play::{AbilityId, CardId, Database, TriggerId},
    mana::{parse_mana, parse_mana_list, Mana, ManaRestriction},
    player::{mana_pool::ManaSource, Controller},
    protogen,
    targets::Restriction,
    triggers::Trigger,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub(crate) struct SorcerySpeed;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub(crate) struct Craft;

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

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub(crate) struct ETBAbilities(pub(crate) Vec<AbilityId>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub(crate) struct ModifiedETBAbilities(pub(crate) Vec<AbilityId>);

#[derive(Debug, Clone, Deref, DerefMut, Component, Default)]
pub(crate) struct StaticAbilities(pub(crate) Vec<StaticAbility>);

#[derive(Debug, Clone, Deref, DerefMut, Component, Default)]
pub(crate) struct ModifiedStaticAbilities(pub(crate) Vec<StaticAbility>);

#[derive(Debug, Clone)]
pub(crate) struct AddKeywordsIf {
    pub(crate) keywords: ::counter::Counter<Keyword>,
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::static_ability::AddKeywordsIf> for AddKeywordsIf {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::static_ability::AddKeywordsIf,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            keywords: value
                .keywords
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| Keyword::from_str(s.trim()).with_context(|| anyhow!("Parsing {}", s)))
                .collect::<anyhow::Result<_>>()?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ForceEtbTapped {
    pub(crate) restrictions: Vec<Restriction>,
}

#[derive(Debug, Clone)]
pub(crate) enum StaticAbility {
    AddKeywordsIf(AddKeywordsIf),
    BattlefieldModifier(Box<BattlefieldModifier>),
    CantCastIfAttacked,
    ExtraLandsPerTurn(usize),
    ForceEtbTapped(ForceEtbTapped),
    GreenCannotBeCountered { restrictions: Vec<Restriction> },
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

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut, Default)]
pub(crate) struct ActivatedAbilities(pub(crate) Vec<AbilityId>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut, Default)]
pub(crate) struct ModifiedActivatedAbilities(pub(crate) Vec<AbilityId>);

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub(crate) struct ApplyToSelf;

#[derive(Debug, Clone)]
pub(crate) struct ActivatedAbility {
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
    pub(crate) fn can_be_played_from_hand(
        &self,
        _db: &mut Database,
        _controller: Controller,
    ) -> bool {
        self.effects.iter().any(|effect| effect.effect.cycling())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub(crate) struct Triggers(pub(crate) Vec<TriggerId>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub(crate) struct ModifiedTriggers(pub(crate) Vec<TriggerId>);

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub(crate) struct TriggerListener(pub(crate) CardId);

#[derive(Debug, Clone)]
pub(crate) struct TriggeredAbility {
    pub(crate) trigger: Trigger,
    pub(crate) effects: Vec<AnyEffect>,
    pub(crate) oracle_text: String,
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

#[derive(Debug, PartialEq, Eq, Clone, Component)]
pub(crate) enum GainMana {
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
                gains: parse_mana(&specific.gain)?,
            }),
            protogen::effects::gain_mana::Gain::Choice(choice) => Ok(Self::Choice {
                choices: parse_mana_list(&choice.choices)?,
            }),
        }
    }
}

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub(crate) struct GainManaAbilities(pub(crate) Vec<GainManaAbility>);

#[derive(Debug, Clone, Component)]
pub(crate) struct GainManaAbility {
    pub(crate) cost: AbilityCost,
    pub(crate) gain: GainMana,
    pub(crate) mana_source: Option<ManaSource>,
    pub(crate) mana_restriction: ManaRestriction,
    pub(crate) oracle_text: String,
}

impl TryFrom<&protogen::effects::GainManaAbility> for GainManaAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::GainManaAbility) -> Result<Self, Self::Error> {
        Ok(Self {
            cost: value.cost.get_or_default().try_into()?,
            gain: value.gain_mana.get_or_default().try_into()?,
            mana_source: value
                .mana_source
                .as_ref()
                .map_or(Ok(None), |value| value.try_into().map(Some))?,
            mana_restriction: value.mana_restriction.get_or_default().into(),
            oracle_text: replace_symbols(&value.oracle_text),
        })
    }
}

#[derive(Debug, Clone, Component)]
pub(crate) enum Ability {
    Activated(ActivatedAbility),
    Mana(GainManaAbility),
    Etb { effects: Vec<AnyEffect> },
}

impl Ability {
    pub(crate) fn cost(&self) -> Option<&AbilityCost> {
        match self {
            Ability::Activated(ActivatedAbility { cost, .. })
            | Ability::Mana(GainManaAbility { cost, .. }) => Some(cost),
            Ability::Etb { .. } => None,
        }
    }

    pub(crate) fn apply_to_self(&self) -> bool {
        match self {
            Ability::Activated(ActivatedAbility { apply_to_self, .. }) => *apply_to_self,
            Ability::Mana(_) => false,
            Ability::Etb { .. } => false,
        }
    }

    pub(crate) fn into_effects(self) -> Vec<AnyEffect> {
        match self {
            Ability::Activated(ActivatedAbility { effects, .. }) => effects,
            Ability::Mana(_) => vec![],
            Ability::Etb { effects, .. } => effects,
        }
    }
}
