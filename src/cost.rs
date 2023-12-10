use std::collections::HashSet;

use anyhow::anyhow;
use bevy_ecs::component::Component;

use crate::{card::Color, mana::Mana, protogen};

#[derive(Debug, Clone, PartialEq, Eq, Default, Component)]
pub struct CastingCost {
    pub mana_cost: Vec<Mana>,
}

impl CastingCost {
    pub fn colors(&self) -> HashSet<Color> {
        self.mana_cost.iter().map(|mana| mana.color()).collect()
    }
}

impl TryFrom<&protogen::cost::CastingCost> for CastingCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::CastingCost) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: value
                .mana_costs
                .iter()
                .map(Mana::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdditionalCost {
    SacrificeThis,
}

impl TryFrom<&protogen::cost::AdditionalCost> for AdditionalCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::AdditionalCost) -> Result<Self, Self::Error> {
        value
            .cost
            .as_ref()
            .ok_or_else(|| anyhow!("Expected additional cost to have a cost specified"))
            .map(Self::from)
    }
}

impl From<&protogen::cost::additional_cost::Cost> for AdditionalCost {
    fn from(value: &protogen::cost::additional_cost::Cost) -> Self {
        match value {
            protogen::cost::additional_cost::Cost::SacrificeThis(_) => Self::SacrificeThis,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
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
                .map(Mana::try_from)
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
