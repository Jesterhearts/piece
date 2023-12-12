use std::collections::{HashMap, HashSet};

use anyhow::anyhow;
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};

use crate::{
    controller::ControllerRestriction,
    cost::AbilityCost,
    effects::{
        AnyEffect, BattlefieldModifier, Mill, ReturnFromGraveyardToBattlefield,
        ReturnFromGraveyardToLibrary, TutorLibrary,
    },
    in_play::{AbilityId, CardId, TriggerId},
    mana::Mana,
    protogen,
    targets::Restriction,
    triggers::Trigger,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enchant {
    pub modifiers: Vec<BattlefieldModifier>,
    pub restrictions: Vec<Restriction>,
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
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct ETBAbilities(pub Vec<ETBAbility>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct ModifiedETBAbilities(pub Vec<ETBAbility>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ETBAbility {
    CopyOfAnyCreature,
    Mill(Mill),
    ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary),
    ReturnFromGraveyardToBattlefield(ReturnFromGraveyardToBattlefield),
    TutorLibrary(TutorLibrary),
}

impl TryFrom<&protogen::abilities::ETBAbility> for ETBAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::ETBAbility) -> Result<Self, Self::Error> {
        value
            .ability
            .as_ref()
            .ok_or_else(|| anyhow!("Expected etb ability to have an ability specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::abilities::etbability::Ability> for ETBAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::abilities::etbability::Ability) -> Result<Self, Self::Error> {
        match value {
            protogen::abilities::etbability::Ability::CopyOfAnyCreature(_) => {
                Ok(Self::CopyOfAnyCreature)
            }
            protogen::abilities::etbability::Ability::Mill(mill) => {
                Ok(Self::Mill(mill.try_into()?))
            }
            protogen::abilities::etbability::Ability::ReturnFromGraveyardToLibrary(ret) => {
                Ok(Self::ReturnFromGraveyardToLibrary(ret.try_into()?))
            }
            protogen::abilities::etbability::Ability::ReturnFromGraveyardToBattlefield(ret) => {
                Ok(Self::ReturnFromGraveyardToBattlefield(ret.try_into()?))
            }
            protogen::abilities::etbability::Ability::TutorLibrary(tutor) => {
                Ok(Self::TutorLibrary(tutor.try_into()?))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component, Default)]
pub struct StaticAbilities(pub Vec<StaticAbility>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component, Default)]
pub struct ModifiedStaticAbilities(pub Vec<StaticAbility>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticAbility {
    GreenCannotBeCountered { controller: ControllerRestriction },
    BattlefieldModifier(BattlefieldModifier),
    ExtraLandsPerTurn(usize),
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
                Ok(Self::BattlefieldModifier(modifier.try_into()?))
            }
            protogen::abilities::static_ability::Ability::ExtraLandsPerTurn(extra_lands) => {
                Ok(Self::ExtraLandsPerTurn(usize::try_from(extra_lands.count)?))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivatedAbility {
    pub cost: AbilityCost,
    pub effects: Vec<AnyEffect>,
    pub apply_to_self: bool,
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
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct Triggers(pub Vec<TriggerId>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct ModifiedTriggers(pub Vec<TriggerId>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct TriggerListeners(pub HashSet<CardId>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggeredAbility {
    pub trigger: Trigger,
    pub effects: Vec<AnyEffect>,
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
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Component)]
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

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct GainManaAbilities(pub Vec<GainManaAbility>);

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct GainManaAbility {
    pub cost: AbilityCost,
    pub gain: GainMana,
}

impl TryFrom<&protogen::effects::GainManaAbility> for GainManaAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::GainManaAbility) -> Result<Self, Self::Error> {
        Ok(Self {
            cost: value.cost.get_or_default().try_into()?,
            gain: value.gain_mana.get_or_default().try_into()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub enum Ability {
    Activated(ActivatedAbility),
    Mana(GainManaAbility),
}
impl Ability {
    pub fn cost(&self) -> &AbilityCost {
        match self {
            Ability::Activated(ActivatedAbility { cost, .. })
            | Ability::Mana(GainManaAbility { cost, .. }) => cost,
        }
    }

    pub fn apply_to_self(&self) -> bool {
        match self {
            Ability::Activated(ActivatedAbility { apply_to_self, .. }) => *apply_to_self,
            Ability::Mana(_) => false,
        }
    }

    pub fn into_effects(self) -> Vec<AnyEffect> {
        match self {
            Ability::Activated(ActivatedAbility { effects, .. }) => effects,
            Ability::Mana(_) => vec![],
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
