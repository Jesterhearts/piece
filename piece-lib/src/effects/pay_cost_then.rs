use crate::{
    abilities::Ability,
    effects::EffectBehaviors,
    pending_results::pay_costs::{Cost, PayCost, SacrificePermanent, SpendMana, TapPermanent},
    player::mana_pool::SpendReason,
    protogen::{cost::additional_cost, effects::PayCostThen},
};

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

        for cost in self.cost.additional_costs.iter() {
            match cost.cost.as_ref().unwrap() {
                additional_cost::Cost::DiscardThis(_) => unreachable!(),
                additional_cost::Cost::ExileCardsCmcX(_) => unreachable!(),
                additional_cost::Cost::SacrificeSource(_) => unreachable!(),
                additional_cost::Cost::PayLife(_) => todo!(),
                additional_cost::Cost::ExileCard(_) => todo!(),
                additional_cost::Cost::ExileXOrMoreCards(_) => todo!(),
                additional_cost::Cost::ExileSharingCardType(_) => todo!(),
                additional_cost::Cost::TapPermanentsPowerXOrMore(_) => todo!(),
                additional_cost::Cost::RemoveCounters(_) => todo!(),
                additional_cost::Cost::SacrificePermanent(sacrifice) => {
                    results.push_pay_costs(PayCost::new(
                        source,
                        Cost::SacrificePermanent(SacrificePermanent::new(
                            sacrifice.restrictions.clone(),
                        )),
                    ));
                }
                additional_cost::Cost::TapPermanent(tap) => {
                    results.push_pay_costs(PayCost::new(
                        source,
                        Cost::TapPermanent(TapPermanent::new(tap.restrictions.clone())),
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

        for cost in self.cost.additional_costs.iter() {
            match cost.cost.as_ref().unwrap() {
                additional_cost::Cost::DiscardThis(_) => unreachable!(),
                additional_cost::Cost::ExileCardsCmcX(_) => unreachable!(),
                additional_cost::Cost::SacrificeSource(_) => unreachable!(),
                additional_cost::Cost::PayLife(_) => todo!(),
                additional_cost::Cost::ExileCard(_) => todo!(),
                additional_cost::Cost::ExileXOrMoreCards(_) => todo!(),
                additional_cost::Cost::ExileSharingCardType(_) => todo!(),
                additional_cost::Cost::TapPermanentsPowerXOrMore(_) => todo!(),
                additional_cost::Cost::RemoveCounters(_) => todo!(),
                additional_cost::Cost::SacrificePermanent(sacrifice) => {
                    results.push_pay_costs(PayCost::new(
                        source,
                        Cost::SacrificePermanent(SacrificePermanent::new(
                            sacrifice.restrictions.clone(),
                        )),
                    ));
                }
                additional_cost::Cost::TapPermanent(tap) => {
                    results.push_pay_costs(PayCost::new(
                        source,
                        Cost::TapPermanent(TapPermanent::new(tap.restrictions.clone())),
                    ));
                }
            }
        }

        results.add_ability_to_stack(source, Ability::EtbOrTriggered(self.effects.clone()));
    }
}
