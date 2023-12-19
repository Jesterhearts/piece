use std::collections::HashSet;

use anyhow::anyhow;
use bevy_ecs::component::Component;
use itertools::Itertools;

use crate::{card::Color, mana::ManaCost, protogen, targets::Restriction};

#[derive(Debug, Clone, PartialEq, Eq, Default, Component)]
pub struct CastingCost {
    pub mana_cost: Vec<ManaCost>,
}

impl CastingCost {
    pub fn colors(&self) -> HashSet<Color> {
        self.mana_cost.iter().map(|mana| mana.color()).collect()
    }

    pub fn text(&self) -> String {
        let mut result = String::default();

        for mana in self.mana_cost.iter() {
            mana.push_mana_symbol(&mut result);
        }

        result
    }

    pub(crate) fn cmc(&self) -> usize {
        self.mana_cost
            .iter()
            .map(|mana| match mana {
                &ManaCost::Generic(count) => count,
                _ => 1,
            })
            .sum::<usize>()
    }
}

impl TryFrom<&protogen::cost::CastingCost> for CastingCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::CastingCost) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: value
                .mana_costs
                .iter()
                .map(ManaCost::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayLife {
    pub count: usize,
}

impl TryFrom<&protogen::cost::additional_cost::PayLife> for PayLife {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::additional_cost::PayLife) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdditionalCost {
    SacrificeThis,
    PayLife(PayLife),
    SacrificePermanent(Vec<Restriction>),
}

impl AdditionalCost {
    pub fn text(&self) -> String {
        match self {
            AdditionalCost::SacrificeThis => "Sacrifice this".to_string(),
            AdditionalCost::PayLife(pay) => format!("Pay {} life", pay.count),
            AdditionalCost::SacrificePermanent(restrictions) => {
                format!(
                    "Sacrifice a {}",
                    restrictions.iter().map(|r| r.text()).join(", ")
                )
            }
        }
    }
}

impl TryFrom<&protogen::cost::AdditionalCost> for AdditionalCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::AdditionalCost) -> Result<Self, Self::Error> {
        value
            .cost
            .as_ref()
            .ok_or_else(|| anyhow!("Expected additional cost to have a cost specified"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::cost::additional_cost::Cost> for AdditionalCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::additional_cost::Cost) -> Result<Self, Self::Error> {
        match value {
            protogen::cost::additional_cost::Cost::SacrificeThis(_) => Ok(Self::SacrificeThis),
            protogen::cost::additional_cost::Cost::PayLife(pay) => {
                Ok(Self::PayLife(pay.try_into()?))
            }
            protogen::cost::additional_cost::Cost::SacrificePermanent(sacrifice) => {
                Ok(Self::SacrificePermanent(
                    sacrifice
                        .restrictions
                        .iter()
                        .map(Restriction::try_from)
                        .collect::<anyhow::Result<_>>()?,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct AbilityCost {
    pub mana_cost: Vec<ManaCost>,
    pub tap: bool,
    pub additional_cost: Vec<AdditionalCost>,
}
impl AbilityCost {
    pub fn text(&self) -> String {
        std::iter::once(
            self.mana_cost
                .iter()
                .map(|c| {
                    let mut result = String::default();
                    c.push_mana_symbol(&mut result);
                    result
                })
                .join(""),
        )
        .filter(|t| !t.is_empty())
        .chain(
            std::iter::once(self.tap)
                .filter(|t| *t)
                .map(|_| "↩".to_string()),
        )
        .chain(self.additional_cost.iter().map(|cost| cost.text()))
        .join(", ")
    }
}

impl TryFrom<&protogen::cost::AbilityCost> for AbilityCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::AbilityCost) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: value
                .mana_costs
                .iter()
                .map(ManaCost::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            tap: value.tap.unwrap_or_default(),
            additional_cost: value
                .additional_costs
                .iter()
                .map(AdditionalCost::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}
