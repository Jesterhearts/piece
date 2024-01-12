use std::collections::HashSet;

use itertools::Itertools;
use tracing::Level;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::{
        effects::{effect::Effect, CopyOfAnyCreatureNonTargeting},
        targets::Location,
        types::Type,
    },
    stack::ActiveTarget,
    types::TypeSet,
};

impl EffectBehaviors for CopyOfAnyCreatureNonTargeting {
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
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        _controller: crate::player::Controller,
        already_chosen: &HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for creature in db.cards.keys().filter(|card| {
            card.passes_restrictions(db, log_session, source, &card.faceup_face(db).restrictions)
                && card.is_in_location(db, Location::ON_BATTLEFIELD)
                && card.types_intersect(db, &TypeSet::from([Type::CREATURE]))
        }) {
            let target = ActiveTarget::Battlefield { id: *creature };
            if already_chosen.contains(&target) {
                continue;
            }
            targets.push(target);
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

    #[instrument(level = Level::INFO, skip(_db, results))]
    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if let Ok(target) = targets.into_iter().exactly_one() {
            results.push_settled(ActionResult::CloneCreatureNonTargeting { source, target })
        } else {
            warn!("Skipping targets");
        }
    }
}
