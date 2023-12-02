use anyhow::anyhow;

use crate::{mana::Mana, protogen};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CastingCost {
    pub mana_cost: Vec<Mana>,
}

impl TryFrom<&protogen::cost::CastingCost> for CastingCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::CastingCost) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: value
                .mana_costs
                .iter()
                .map(|cost| {
                    cost.cost
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected cost to have a cost specified"))
                        .and_then(Mana::try_from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AdditionalCost {
    SacrificeThis,
}

impl From<&protogen::cost::additional_cost::Cost> for AdditionalCost {
    fn from(value: &protogen::cost::additional_cost::Cost) -> Self {
        match value {
            protogen::cost::additional_cost::Cost::SacrificeThis(_) => Self::SacrificeThis,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbilityCost {
    pub mana_cost: Vec<Mana>,
    pub tap: bool,
    pub additional_cost: Vec<AdditionalCost>,
}

impl TryFrom<&protogen::cost::AbilityCost> for AbilityCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::AbilityCost) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: value
                .mana_costs
                .iter()
                .map(|cost| {
                    cost.cost
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected cost to have a cost specified"))
                        .and_then(Mana::try_from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            tap: value.tap.unwrap_or_default(),
            additional_cost: value
                .additional_costs
                .iter()
                .map(|additional_cost| {
                    additional_cost
                        .cost
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected additional cost to have a cost specified"))
                        .map(AdditionalCost::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}