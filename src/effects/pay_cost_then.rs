use crate::{
    abilities::Ability,
    cost::{AbilityCost, AdditionalCost},
    effects::{AnyEffect, EffectBehaviors},
    pending_results::pay_costs::{Cost, PayCost, SacrificePermanent, SpendMana, TapPermanent},
    player::mana_pool::SpendReason,
    protogen,
};

#[derive(Debug, Clone)]
pub(crate) struct PayCostThen {
    cost: AbilityCost,
    effects: Vec<AnyEffect>,
}

impl TryFrom<&protogen::effects::PayCostThen> for PayCostThen {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::PayCostThen) -> Result<Self, Self::Error> {
        Ok(Self {
            cost: value.cost.get_or_default().try_into()?,
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for PayCostThen {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_pay_costs(PayCost::new(
            source,
            Cost::SpendMana(SpendMana::new(
                self.cost.mana_cost.clone(),
                SpendReason::Other,
            )),
        ));

        for cost in self.cost.additional_cost.iter() {
            match cost {
                AdditionalCost::DiscardThis => unreachable!(),
                AdditionalCost::ExileCardsCmcX(_) => unreachable!(),
                AdditionalCost::SacrificeSource => unreachable!(),
                AdditionalCost::PayLife(_) => todo!(),
                AdditionalCost::ExileCard { .. } => todo!(),
                AdditionalCost::ExileXOrMoreCards { .. } => todo!(),
                AdditionalCost::ExileSharingCardType { .. } => todo!(),
                AdditionalCost::TapPermanentsPowerXOrMore { .. } => todo!(),
                AdditionalCost::RemoveCounter { .. } => todo!(),
                AdditionalCost::SacrificePermanent(restrictions) => {
                    results.push_pay_costs(PayCost::new(
                        source,
                        Cost::SacrificePermanent(SacrificePermanent::new(restrictions.clone())),
                    ));
                }
                AdditionalCost::TapPermanent(restrictions) => {
                    results.push_pay_costs(PayCost::new(
                        source,
                        Cost::TapPermanent(TapPermanent::new(restrictions.clone())),
                    ));
                }
            }
        }

        results.add_ability_to_stack(source, Ability::EtbOrTriggered(self.effects.clone()));
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_pay_costs(PayCost::new(
            source,
            Cost::SpendMana(SpendMana::new(
                self.cost.mana_cost.clone(),
                SpendReason::Other,
            )),
        ));

        for cost in self.cost.additional_cost.iter() {
            match cost {
                AdditionalCost::DiscardThis => unreachable!(),
                AdditionalCost::ExileCardsCmcX(_) => unreachable!(),
                AdditionalCost::SacrificeSource => unreachable!(),
                AdditionalCost::PayLife(_) => todo!(),
                AdditionalCost::ExileCard { .. } => todo!(),
                AdditionalCost::ExileXOrMoreCards { .. } => todo!(),
                AdditionalCost::ExileSharingCardType { .. } => todo!(),
                AdditionalCost::TapPermanentsPowerXOrMore { .. } => todo!(),
                AdditionalCost::RemoveCounter { .. } => todo!(),
                AdditionalCost::SacrificePermanent(restrictions) => {
                    results.push_pay_costs(PayCost::new(
                        source,
                        Cost::SacrificePermanent(SacrificePermanent::new(restrictions.clone())),
                    ));
                }
                AdditionalCost::TapPermanent(restrictions) => {
                    results.push_pay_costs(PayCost::new(
                        source,
                        Cost::TapPermanent(TapPermanent::new(restrictions.clone())),
                    ));
                }
            }
        }

        results.add_ability_to_stack(source, Ability::EtbOrTriggered(self.effects.clone()));
    }
}
