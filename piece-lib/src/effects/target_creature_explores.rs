use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    log::LogId,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::effects::{effect::Effect, TargetCreatureExplores},
    protogen::types::Type,
    stack::ActiveTarget,
    types::TypeSet,
};

impl EffectBehaviors for TargetCreatureExplores {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
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
        _source: crate::in_play::CardId,
        _log_session: crate::log::LogId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        db.battlefield[controller]
            .iter()
            .filter(|card| {
                card.types_intersect(db, &TypeSet::from([Type::CREATURE]))
                    && card.can_be_targeted(db, controller)
            })
            .map(|card| ActiveTarget::Battlefield { id: *card })
            .filter(|target| !already_chosen.contains(target))
            .collect_vec()
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

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let target = targets.into_iter().exactly_one().unwrap();

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
        results.push_settled(ActionResult::Explore { target })
    }
}
