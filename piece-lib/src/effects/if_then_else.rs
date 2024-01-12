use crate::{effects::EffectBehaviors, log::LogId, protogen::effects::IfThenElse};

impl EffectBehaviors for IfThenElse {
    fn needs_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
    ) -> usize {
        if source.passes_restrictions(db, LogId::current(db), source, &self.if_) {
            self.then.effect.as_ref().unwrap().needs_targets(db, source)
        } else {
            self.else_
                .effect
                .as_ref()
                .unwrap()
                .needs_targets(db, source)
        }
    }

    fn wants_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
    ) -> usize {
        if source.passes_restrictions(db, LogId::current(db), source, &self.if_) {
            self.then.effect.as_ref().unwrap().wants_targets(db, source)
        } else {
            self.else_
                .effect
                .as_ref()
                .unwrap()
                .wants_targets(db, source)
        }
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        if source.passes_restrictions(db, log_session, source, &self.if_) {
            self.then.effect.as_ref().unwrap().valid_targets(
                db,
                source,
                crate::log::LogId::current(db),
                controller,
                already_chosen,
            )
        } else {
            self.else_.effect.as_ref().unwrap().valid_targets(
                db,
                source,
                crate::log::LogId::current(db),
                controller,
                already_chosen,
            )
        }
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if source.passes_restrictions(db, LogId::current(db), source, &self.if_) {
            self.then
                .effect
                .as_ref()
                .unwrap()
                .push_pending_behavior(db, source, controller, results)
        } else {
            self.else_
                .effect
                .as_ref()
                .unwrap()
                .push_pending_behavior(db, source, controller, results)
        }
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if source.passes_restrictions(db, LogId::current(db), source, &self.if_) {
            self.then
                .effect
                .as_ref()
                .unwrap()
                .push_behavior_with_targets(db, targets, apply_to_self, source, controller, results)
        } else {
            self.else_
                .effect
                .as_ref()
                .unwrap()
                .push_behavior_with_targets(db, targets, apply_to_self, source, controller, results)
        }
    }
}
