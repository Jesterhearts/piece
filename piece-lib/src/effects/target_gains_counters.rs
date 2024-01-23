use std::collections::HashSet;

use itertools::Itertools;
use tracing::Level;

use crate::{
    action_result::{add_counters::AddCounters, ActionResult},
    effects::EffectBehaviors,
    log::LogId,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::effects::{effect::Effect, TargetGainsCounters},
    stack::ActiveTarget,
};

impl EffectBehaviors for TargetGainsCounters {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        if self.is_optional {
            0
        } else {
            1
        }
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in db.cards.keys() {
            if card.passes_restrictions(db, log_session, source, &self.restrictions)
                && card.passes_restrictions(
                    db,
                    log_session,
                    source,
                    &source.faceup_face(db).restrictions,
                )
                && card.can_be_targeted(db, controller)
            {
                let target = card.target_from_location(db).unwrap();
                if already_chosen.contains(&target) {
                    continue;
                }
                targets.push(target);
            }
        }

        targets
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
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
            source,
        ));
    }

    #[instrument(level = Level::INFO, skip(db, results))]
    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if let Ok(target) = targets.into_iter().exactly_one() {
            if !self
                .valid_targets(
                    db,
                    source,
                    LogId::current(db),
                    controller,
                    &HashSet::default(),
                )
                .into_iter()
                .any(|t| t == target)
            {
                return;
            }

            let target = match target {
                ActiveTarget::Battlefield { id } => id,
                ActiveTarget::Graveyard { id } => id,
                _ => unreachable!(),
            };

            results.push_settled(ActionResult::from(AddCounters {
                source,
                target,
                count: self.count.count.as_ref().unwrap().clone(),
                counter: self.counter,
            }));
        } else {
            warn!("Skipping targets");
        }
    }
}
