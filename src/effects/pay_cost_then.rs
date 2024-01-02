use crate::{
    abilities::Ability,
    battlefield::pay_costs::{PayCost, SacrificePermanent, SpendMana, TapPermanent},
    cost::{AbilityCost, AdditionalCost},
    effects::{AnyEffect, EffectBehaviors},
    in_play::AbilityId,
    player::mana_pool::SpendReason,
    protogen,
};

#[derive(Debug)]
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
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_pay_costs(PayCost::SpendMana(SpendMana::new(
            self.cost.mana_cost.clone(),
            source,
            SpendReason::Other,
        )));

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
                    results.push_pay_costs(PayCost::SacrificePermanent(SacrificePermanent::new(
                        restrictions.clone(),
                        source,
                    )));
                }
                AdditionalCost::TapPermanent(restrictions) => {
                    results.push_pay_costs(PayCost::TapPermanent(TapPermanent::new(
                        restrictions.clone(),
                        source,
                    )));
                }
            }
        }

        results.add_ability_to_stack(AbilityId::upload_ability(
            db,
            source,
            Ability::Etb {
                effects: self.effects.clone(),
            },
        ))
    }

    fn push_behavior_with_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_pay_costs(PayCost::SpendMana(SpendMana::new(
            self.cost.mana_cost.clone(),
            source,
            SpendReason::Other,
        )));

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
                    results.push_pay_costs(PayCost::SacrificePermanent(SacrificePermanent::new(
                        restrictions.clone(),
                        source,
                    )));
                }
                AdditionalCost::TapPermanent(restrictions) => {
                    results.push_pay_costs(PayCost::TapPermanent(TapPermanent::new(
                        restrictions.clone(),
                        source,
                    )));
                }
            }
        }

        results.add_ability_to_stack(AbilityId::upload_ability(
            db,
            source,
            Ability::Etb {
                effects: self.effects.clone(),
            },
        ))
    }
}
