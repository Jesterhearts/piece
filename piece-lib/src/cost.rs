use anyhow::anyhow;

use crate::protogen::{
    self, color::Color, cost::ManaCost, counters::Counter, targets::Restriction,
};

#[derive(Debug, Clone, Default)]
pub struct CastingCost {
    pub mana_cost: Vec<protobuf::EnumOrUnknown<ManaCost>>,
    pub(crate) additional_cost: Vec<AdditionalCost>,
}

impl CastingCost {
    pub(crate) fn colors(&self) -> Vec<Color> {
        self.mana_cost
            .iter()
            .map(|mana| mana.enum_value().unwrap().color())
            .collect()
    }

    pub fn text(&self) -> String {
        let mut result = String::default();

        let generic = self
            .mana_cost
            .iter()
            .filter(|cost| matches!(cost.enum_value().unwrap(), ManaCost::GENERIC))
            .count();

        let mut pushed_generic = false;
        for mana in self.mana_cost.iter() {
            match mana.enum_value().unwrap() {
                ManaCost::WHITE => result.push('\u{e600}'),
                ManaCost::BLUE => result.push('\u{e601}'),
                ManaCost::BLACK => result.push('\u{e602}'),
                ManaCost::RED => result.push('\u{e603}'),
                ManaCost::GREEN => result.push('\u{e604}'),
                ManaCost::COLORLESS => result.push('\u{e904}'),
                ManaCost::GENERIC => {
                    if !pushed_generic {
                        match generic {
                            0 => result.push('\u{e605}'),
                            1 => result.push('\u{e606}'),
                            2 => result.push('\u{e607}'),
                            3 => result.push('\u{e608}'),
                            4 => result.push('\u{e609}'),
                            5 => result.push('\u{e60a}'),
                            6 => result.push('\u{e60b}'),
                            7 => result.push('\u{e60c}'),
                            8 => result.push('\u{e60d}'),
                            9 => result.push('\u{e60e}'),
                            10 => result.push('\u{e60f}'),
                            11 => result.push('\u{e610}'),
                            12 => result.push('\u{e611}'),
                            13 => result.push('\u{e612}'),
                            14 => result.push('\u{e613}'),
                            15 => result.push('\u{e614}'),
                            16 => result.push('\u{e62a}'),
                            17 => result.push('\u{e62b}'),
                            18 => result.push('\u{e62c}'),
                            19 => result.push('\u{e62d}'),
                            20 => result.push('\u{e62e}'),
                            _ => result.push_str(&format!("{}", generic)),
                        }
                        pushed_generic = true;
                    }
                }
                ManaCost::X => result.push('\u{e615}'),
                ManaCost::TWO_X => result.push_str("\u{e615}\u{e615}"),
            }
        }

        result
    }

    pub(crate) fn cmc(&self) -> usize {
        self.mana_cost.len()
    }
}

impl TryFrom<&protogen::cost::CastingCost> for CastingCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::CastingCost) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: value.mana_cost.clone(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
        counter: protobuf::EnumOrUnknown<Counter>,
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
                Ok(Self::SacrificePermanent(sacrifice.restrictions.clone()))
            }
            protogen::cost::additional_cost::Cost::TapPermanent(tap) => {
                Ok(Self::TapPermanent(tap.restrictions.clone()))
            }
            protogen::cost::additional_cost::Cost::TapPermanentsPowerXOrMore(tap) => {
                Ok(Self::TapPermanentsPowerXOrMore {
                    x_is: usize::try_from(tap.x_is)?,
                    restrictions: tap.restrictions.clone(),
                })
            }
            protogen::cost::additional_cost::Cost::ExileCardsCmcX(value) => {
                Ok(Self::ExileCardsCmcX(value.restrictions.clone()))
            }
            protogen::cost::additional_cost::Cost::ExileCard(value) => Ok(Self::ExileCard {
                restrictions: value.restrictions.clone(),
            }),
            protogen::cost::additional_cost::Cost::ExileSharingCardType(value) => {
                Ok(Self::ExileSharingCardType {
                    count: usize::try_from(value.count)?,
                })
            }
            protogen::cost::additional_cost::Cost::ExileXOrMoreCards(value) => {
                Ok(Self::ExileXOrMoreCards {
                    minimum: usize::try_from(value.minimum)?,
                    restrictions: value.restrictions.clone(),
                })
            }
            protogen::cost::additional_cost::Cost::RemoveCounters(value) => {
                Ok(Self::RemoveCounter {
                    counter: value.counter,
                    count: usize::try_from(value.count)?,
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AbilityRestriction {
    AttackedWithXOrMoreCreatures(usize),
    OncePerTurn,
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
            protogen::cost::ability_restriction::Restriction::OncePerTurn(_) => {
                Ok(Self::OncePerTurn)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AbilityCost {
    pub(crate) mana_cost: Vec<protobuf::EnumOrUnknown<ManaCost>>,
    pub(crate) tap: bool,
    pub(crate) additional_cost: Vec<AdditionalCost>,
    pub(crate) restrictions: Vec<AbilityRestriction>,
}

impl TryFrom<&protogen::cost::AbilityCost> for AbilityCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::AbilityCost) -> Result<Self, Self::Error> {
        Ok(Self {
            mana_cost: value.mana_cost.clone(),
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
    pub(crate) reduction: Vec<protobuf::EnumOrUnknown<ManaCost>>,
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
            reduction: value.reduction.clone(),
        })
    }
}
