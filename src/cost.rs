use std::collections::HashSet;

use anyhow::anyhow;
use bevy_ecs::component::Component;
use itertools::Itertools;

use crate::{
    card::Color,
    in_play::{CardId, Database},
    mana::ManaCost,
    protogen,
    targets::Restriction,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Component)]
pub struct CastingCost {
    pub mana_cost: Vec<ManaCost>,
    pub additional_cost: Vec<AdditionalCost>,
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
            additional_cost: value
                .additional_costs
                .iter()
                .map(AdditionalCost::try_from)
                .collect::<anyhow::Result<_>>()?,
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
    DiscardThis,
    ExileCard {
        restrictions: Vec<Restriction>,
    },
    ExileXOrMoreCards {
        minimum: usize,
        restrictions: Vec<Restriction>,
    },
    ExileSharingCardType {
        count: usize,
    },
    ExileCardsCmcX(Vec<Restriction>),
    SacrificeSource,
    PayLife(PayLife),
    SacrificePermanent(Vec<Restriction>),
    TapPermanent(Vec<Restriction>),
}

impl AdditionalCost {
    pub fn text(&self, db: &Database, source: CardId) -> String {
        match self {
            AdditionalCost::DiscardThis => format!("discard {}", source.name(db)),
            AdditionalCost::SacrificeSource => format!("Sacrifice {}", source.name(db)),
            AdditionalCost::PayLife(pay) => format!("Pay {} life", pay.count),
            AdditionalCost::SacrificePermanent(restrictions) => {
                format!(
                    "Sacrifice {}",
                    restrictions.iter().map(|r| r.text()).join(", ")
                )
            }
            AdditionalCost::TapPermanent(tap) => {
                format!("Tap {}", tap.iter().map(|t| t.text()).join(", "))
            }
            AdditionalCost::ExileCardsCmcX(restrictions) => format!(
                "Exile one or more {} cards with cmc X",
                restrictions.iter().map(|r| r.text()).join(", ")
            ),
            AdditionalCost::ExileCard { restrictions } => {
                format!("Exile {}", restrictions.iter().map(|r| r.text()).join(", "))
            }
            AdditionalCost::ExileXOrMoreCards {
                minimum,
                restrictions,
            } => format!(
                "Exile {} or more {}",
                minimum,
                restrictions.iter().map(|r| r.text()).join(", ")
            ),
            AdditionalCost::ExileSharingCardType { count } => {
                format!("Exile {} cards sharing a card type", count)
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
            protogen::cost::additional_cost::Cost::DiscardThis(_) => Ok(Self::DiscardThis),
            protogen::cost::additional_cost::Cost::SacrificeSource(_) => Ok(Self::SacrificeSource),
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
            protogen::cost::additional_cost::Cost::TapPermanent(tap) => Ok(Self::TapPermanent(
                tap.restrictions
                    .iter()
                    .map(Restriction::try_from)
                    .collect::<anyhow::Result<_>>()?,
            )),
            protogen::cost::additional_cost::Cost::ExileCardsCmcX(value) => {
                Ok(Self::ExileCardsCmcX(
                    value
                        .restrictions
                        .iter()
                        .map(Restriction::try_from)
                        .collect::<anyhow::Result<_>>()?,
                ))
            }
            protogen::cost::additional_cost::Cost::ExileCard(value) => Ok(Self::ExileCard {
                restrictions: value
                    .restrictions
                    .iter()
                    .map(Restriction::try_from)
                    .collect::<anyhow::Result<_>>()?,
            }),
            protogen::cost::additional_cost::Cost::ExileSharingCardType(value) => {
                Ok(Self::ExileSharingCardType {
                    count: usize::try_from(value.count)?,
                })
            }
            protogen::cost::additional_cost::Cost::ExileXOrMoreCards(value) => {
                Ok(Self::ExileXOrMoreCards {
                    minimum: usize::try_from(value.minimum)?,
                    restrictions: value
                        .restrictions
                        .iter()
                        .map(Restriction::try_from)
                        .collect::<anyhow::Result<_>>()?,
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct Ward {
    pub mana_cost: Vec<ManaCost>,
}

impl TryFrom<&protogen::cost::Ward> for Ward {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::Ward) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: value
                .mana_costs
                .iter()
                .map(ManaCost::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AbilityRestriction {
    AttackedWithXOrMoreCreatures(usize),
}

impl TryFrom<&protogen::cost::AbilityRestriction> for AbilityRestriction {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::AbilityRestriction) -> Result<Self, Self::Error> {
        value
            .restriction
            .as_ref()
            .ok_or_else(|| anyhow!("Expected ability restriction to have a restriction set"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::cost::ability_restriction::Restriction> for AbilityRestriction {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::cost::ability_restriction::Restriction,
    ) -> Result<Self, Self::Error> {
        match value {
            protogen::cost::ability_restriction::Restriction::AttackedWithXOrMoreCreatures(
                value,
            ) => Ok(Self::AttackedWithXOrMoreCreatures(usize::try_from(
                value.x_is,
            )?)),
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct AbilityCost {
    pub mana_cost: Vec<ManaCost>,
    pub tap: bool,
    pub additional_cost: Vec<AdditionalCost>,
    pub restrictions: Vec<AbilityRestriction>,
}

impl AbilityCost {
    pub fn text(&self, db: &Database, source: CardId) -> String {
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
        .chain(
            self.additional_cost
                .iter()
                .map(|cost| cost.text(db, source)),
        )
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
            restrictions: value
                .restrictions
                .iter()
                .map(AbilityRestriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XIs {
    Cmc,
}

impl TryFrom<&protogen::cost::XIs> for XIs {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::XIs) -> Result<Self, Self::Error> {
        value
            .x_is
            .as_ref()
            .ok_or_else(|| anyhow!("Expected xis to have an x set"))
            .map(Self::from)
    }
}

impl From<&protogen::cost::xis::X_is> for XIs {
    fn from(value: &protogen::cost::xis::X_is) -> Self {
        match value {
            protogen::cost::xis::X_is::Cmc(_) => Self::Cmc,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReduceWhen {
    TargetTappedCreature,
}

impl From<&protogen::cost::cost_reducer::When> for ReduceWhen {
    fn from(value: &protogen::cost::cost_reducer::When) -> Self {
        match value {
            protogen::cost::cost_reducer::When::TargetTappedCreature(_) => {
                Self::TargetTappedCreature
            }
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct CostReducer {
    pub when: ReduceWhen,
    pub reduction: ManaCost,
}

impl TryFrom<&protogen::cost::CostReducer> for CostReducer {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::CostReducer) -> Result<Self, Self::Error> {
        Ok(Self {
            when: value
                .when
                .as_ref()
                .ok_or_else(|| anyhow!("Expected reducer to have a when set"))
                .map(ReduceWhen::from)?,
            reduction: value.reduction.get_or_default().try_into()?,
        })
    }
}
