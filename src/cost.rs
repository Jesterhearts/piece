use std::collections::HashSet;

use anyhow::anyhow;
use itertools::Itertools;

use crate::{
    card::{replace_symbols, Color},
    counters::Counter,
    mana::ManaCost,
    protogen,
    targets::Restriction,
};

#[derive(Debug, Clone, Default)]
pub struct CastingCost {
    pub cost_string: String,
    pub(crate) mana_cost: Vec<ManaCost>,
    pub(crate) additional_cost: Vec<AdditionalCost>,
}

impl CastingCost {
    pub(crate) fn colors(&self) -> HashSet<Color> {
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
            cost_string: replace_symbols(&value.mana_cost),
            mana_cost: parse_mana_cost(&value.mana_cost)?,
            additional_cost: value
                .additional_costs
                .iter()
                .map(AdditionalCost::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PayLife {
    pub(crate) count: usize,
}

impl TryFrom<&protogen::cost::additional_cost::PayLife> for PayLife {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::additional_cost::PayLife) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum AdditionalCost {
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
    RemoveCounter {
        counter: Counter,
        count: usize,
    },
    SacrificePermanent(Vec<Restriction>),
    TapPermanent(Vec<Restriction>),
    TapPermanentsPowerXOrMore {
        x_is: usize,
        restrictions: Vec<Restriction>,
    },
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
            protogen::cost::additional_cost::Cost::TapPermanentsPowerXOrMore(tap) => {
                Ok(Self::TapPermanentsPowerXOrMore {
                    x_is: usize::try_from(tap.x_is)?,
                    restrictions: tap
                        .restrictions
                        .iter()
                        .map(Restriction::try_from)
                        .collect::<anyhow::Result<_>>()?,
                })
            }
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
            protogen::cost::additional_cost::Cost::RemoveCounters(value) => {
                Ok(Self::RemoveCounter {
                    counter: (&value.counter).try_into()?,
                    count: usize::try_from(value.count)?,
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ward {
    pub(crate) mana_cost: Vec<ManaCost>,
}

impl TryFrom<&protogen::cost::Ward> for Ward {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::Ward) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: parse_mana_cost(&value.mana_cost)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AbilityRestriction {
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

#[derive(Debug, Clone)]
pub(crate) struct AbilityCost {
    pub(crate) mana_cost: Vec<ManaCost>,
    pub(crate) tap: bool,
    pub(crate) additional_cost: Vec<AdditionalCost>,
    pub(crate) restrictions: Vec<AbilityRestriction>,
}

impl TryFrom<&protogen::cost::AbilityCost> for AbilityCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::AbilityCost) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: parse_mana_cost(&value.mana_cost)?,
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
pub(crate) enum XIs {
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
pub(crate) enum ReduceWhen {
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

#[derive(Debug, Clone)]
pub(crate) struct CostReducer {
    pub(crate) when: ReduceWhen,
    pub(crate) reduction: ManaCost,
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

pub(crate) fn parse_mana_cost(s: &str) -> anyhow::Result<Vec<ManaCost>> {
    let split = s
        .split('}')
        .map(|s| s.trim_start_matches('{'))
        .filter(|s| !s.is_empty())
        .collect_vec();

    let mut results = vec![];
    for symbol in split {
        let cost;
        if let Ok(count) = symbol.parse::<usize>() {
            cost = ManaCost::Generic(count);
        } else {
            match symbol {
                "W" => cost = ManaCost::White,
                "U" => cost = ManaCost::Blue,
                "B" => cost = ManaCost::Black,
                "R" => cost = ManaCost::Red,
                "G" => cost = ManaCost::Green,
                "X" => cost = ManaCost::X,
                "C" => cost = ManaCost::Colorless,
                s => {
                    return Err(anyhow!("Invalid mana cost {}", s));
                }
            }
        }

        if matches!(cost, ManaCost::X) && matches!(results.last(), Some(ManaCost::X)) {
            results.pop();
            results.push(ManaCost::TwoX);
        } else {
            results.push(cost);
        }
    }

    Ok(results)
}
