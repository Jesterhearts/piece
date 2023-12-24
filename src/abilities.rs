use std::collections::HashMap;

use anyhow::anyhow;
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    controller::ControllerRestriction,
    cost::AbilityCost,
    effects::{AnyEffect, BattlefieldModifier},
    in_play::{AbilityId, CardId, Database, TriggerId},
    mana::{Mana, ManaRestriction},
    player::mana_pool::ManaSource,
    protogen,
    triggers::Trigger,
    types::Type,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct SorcerySpeed;

#[derive(Debug, Clone)]
pub struct Enchant {
    pub modifiers: Vec<BattlefieldModifier>,
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
pub struct ETBAbilities(pub Vec<AbilityId>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct ModifiedETBAbilities(pub Vec<AbilityId>);

#[derive(Debug, Clone, Deref, DerefMut, Component, Default)]
pub struct StaticAbilities(pub Vec<StaticAbility>);

#[derive(Debug, Clone, Deref, DerefMut, Component, Default)]
pub struct ModifiedStaticAbilities(pub Vec<StaticAbility>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceEtbTapped {
    pub controller: ControllerRestriction,
    pub types: IndexSet<Type>,
}

#[derive(Debug, Clone)]
pub enum StaticAbility {
    BattlefieldModifier(Box<BattlefieldModifier>),
    ExtraLandsPerTurn(usize),
    ForceEtbTapped(ForceEtbTapped),
    GreenCannotBeCountered { controller: ControllerRestriction },
}

impl TryFrom<&protogen::abilities::StaticAbility> for StaticAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::StaticAbility) -> Result<Self, Self::Error> {
        value
            .ability
            .as_ref()
            .ok_or_else(|| anyhow!("Expected ability to have an ability specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::abilities::static_ability::Ability> for StaticAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::static_ability::Ability) -> Result<Self, Self::Error> {
        match value {
            protogen::abilities::static_ability::Ability::GreenCannotBeCountered(ability) => {
                Ok(Self::GreenCannotBeCountered {
                    controller: ability
                        .controller
                        .controller
                        .as_ref()
                        .map(ControllerRestriction::from)
                        .unwrap_or_default(),
                })
            }
            protogen::abilities::static_ability::Ability::BattlefieldModifier(modifier) => {
                Ok(Self::BattlefieldModifier(Box::new(modifier.try_into()?)))
            }
            protogen::abilities::static_ability::Ability::ExtraLandsPerTurn(extra_lands) => {
                Ok(Self::ExtraLandsPerTurn(usize::try_from(extra_lands.count)?))
            }
            protogen::abilities::static_ability::Ability::ForceEtbTapped(force) => {
                Ok(Self::ForceEtbTapped(ForceEtbTapped {
                    controller: force.controller.get_or_default().try_into()?,
                    types: force
                        .types
                        .iter()
                        .map(Type::try_from)
                        .collect::<anyhow::Result<_>>()?,
                }))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut, Default)]
pub struct ActivatedAbilities(pub Vec<AbilityId>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut, Default)]
pub struct ModifiedActivatedAbilities(pub Vec<AbilityId>);

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct ApplyToSelf;

#[derive(Debug, Clone)]
pub struct ActivatedAbility {
    pub cost: AbilityCost,
    pub effects: Vec<AnyEffect>,
    pub apply_to_self: bool,
    pub oracle_text: String,
    pub sorcery_speed: bool,
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
            oracle_text: value.oracle_text.clone(),
            sorcery_speed: value.sorcery_speed,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct Triggers(pub Vec<TriggerId>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct ModifiedTriggers(pub Vec<TriggerId>);

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct TriggerListener(pub CardId);

#[derive(Debug, Clone)]
pub struct TriggeredAbility {
    pub trigger: Trigger,
    pub effects: Vec<AnyEffect>,
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
            oracle_text: value.oracle_text.clone(),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Component)]
pub enum GainMana {
    Specific { gains: Vec<Mana> },
    Choice { choices: Vec<Vec<Mana>> },
}

impl GainMana {
    fn text(&self) -> String {
        match self {
            GainMana::Specific { gains } => {
                let mut result = "Add ".to_string();
                for mana in gains {
                    mana.push_mana_symbol(&mut result);
                }
                result
            }
            GainMana::Choice { choices } => {
                let mut result = "Add one of ".to_string();

                result.push_str(
                    &choices
                        .iter()
                        .map(|choice| {
                            let mut result = String::default();
                            for mana in choice.iter() {
                                mana.push_mana_symbol(&mut result);
                            }
                            result
                        })
                        .join(", "),
                );

                result
            }
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct GainManaAbilities(pub Vec<GainManaAbility>);

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct GainManaAbility {
    pub cost: AbilityCost,
    pub gain: GainMana,
    pub mana_source: Option<ManaSource>,
    pub mana_restriction: ManaRestriction,
}
impl GainManaAbility {
    pub fn text(&self, db: &Database, source: CardId) -> String {
        format!("{}: {}", self.cost.text(db, source), self.gain.text())
    }
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
        })
    }
}

#[derive(Debug, Clone, Component)]
pub enum Ability {
    Activated(ActivatedAbility),
    Mana(GainManaAbility),
    ETB { effects: Vec<AnyEffect> },
}

impl Ability {
    pub fn cost(&self) -> Option<&AbilityCost> {
        match self {
            Ability::Activated(ActivatedAbility { cost, .. })
            | Ability::Mana(GainManaAbility { cost, .. }) => Some(cost),
            Ability::ETB { .. } => None,
        }
    }

    pub fn apply_to_self(&self) -> bool {
        match self {
            Ability::Activated(ActivatedAbility { apply_to_self, .. }) => *apply_to_self,
            Ability::Mana(_) => false,
            Ability::ETB { .. } => false,
        }
    }

    pub fn into_effects(self) -> Vec<AnyEffect> {
        match self {
            Ability::Activated(ActivatedAbility { effects, .. }) => effects,
            Ability::Mana(_) => vec![],
            Ability::ETB { effects, .. } => effects,
        }
    }
}

pub fn compute_mana_gain(mana: &GainMana, mode: Option<usize>) -> Option<HashMap<Mana, usize>> {
    let mut manas: HashMap<Mana, usize> = HashMap::default();
    match mana {
        GainMana::Specific { gains } => {
            for gain in gains.iter() {
                *manas.entry(*gain).or_default() += 1;
            }
        }
        GainMana::Choice { choices } => {
            let Some(mode) = mode else {
                // No mode selected for modal ability.
                return None;
            };

            for gain in choices[mode].iter() {
                *manas.entry(*gain).or_default() += 1;
            }
        }
    };

    Some(manas)
}
