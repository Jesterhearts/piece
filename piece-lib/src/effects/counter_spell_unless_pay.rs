use itertools::Itertools;
use tracing::Level;

use crate::{
    effects::EffectBehaviors,
    pending_results::{
        choose_targets::ChooseTargets,
        pay_costs::{self, PayCost, SpendMana},
        TargetSource,
    },
    player::mana_pool::SpendReason,
    protogen::effects::CounterSpellOrAbility,
    protogen::{
        cost::ManaCost,
        effects::{counter_spell_unless_pay::Cost, effect::Effect, CounterSpellUnlessPay},
    },
    stack::ActiveTarget,
};

impl EffectBehaviors for CounterSpellUnlessPay {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        1
    }

    #[instrument(level = Level::DEBUG, skip(db))]
    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: &crate::protogen::ids::CardId,
        log_session: crate::log::LogId,
        controller: &crate::protogen::ids::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for id in db.stack.entries.iter().filter_map(|(id, entry)| {
            if entry.passes_restrictions(db, log_session, source, &self.restrictions) {
                Some(id)
            } else {
                None
            }
        }) {
            let target = ActiveTarget::Stack { id: id.clone() };
            if !already_chosen.contains(&target) {
                targets.push(target);
            }
        }

        targets
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: &crate::protogen::ids::CardId,
        controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let valid_targets = self.valid_targets(
            db,
            source,
            crate::log::LogId::current(db),
            controller,
            results.all_currently_targeted(),
        );

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            crate::log::LogId::current(db),
            source.clone(),
        ));
    }

    #[instrument(level = Level::INFO, skip(db, results))]
    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _source: &crate::protogen::ids::CardId,
        _controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if let Ok(ActiveTarget::Stack { id }) = targets.iter().exactly_one() {
            match self.cost.as_ref().unwrap() {
                Cost::Fixed(count) => {
                    results.push_pay_costs(PayCost::new_or_else(
                        db.stack.entries.get(id).unwrap().ty.source().clone(),
                        pay_costs::Cost::SpendMana(SpendMana::new(
                            std::iter::repeat(ManaCost::GENERIC.into())
                                .take(count.count as usize)
                                .collect_vec(),
                            SpendReason::Other,
                        )),
                        vec![Effect::from(CounterSpellOrAbility::default())],
                        targets,
                    ));
                }
            }
        } else {
            warn!("Skipping targeting");
        }
    }
}
