use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors},
    in_play::target_from_location,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    stack::ActiveTarget,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub(crate) struct UntapTarget;

impl EffectBehaviors for UntapTarget {
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
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in db
            .battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .filter(|card| {
                card.passes_restrictions(
                    db,
                    log_session,
                    source,
                    &source.faceup_face(db).restrictions,
                )
            })
            .collect_vec()
        {
            let target = target_from_location(db, *card);
            if card.can_be_targeted(db, controller) && !already_chosen.contains(&target) {
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
            TargetSource::Effect(Effect::from(*self)),
            valid_targets,
            crate::log::LogId::current(db),
            source,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let Ok(ActiveTarget::Battlefield { id }) = targets.into_iter().exactly_one() else {
            unreachable!()
        };
        results.push_settled(ActionResult::Untap(id));
    }
}
