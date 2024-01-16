use crate::{effects::EffectBehaviors, log::LogId, protogen::effects::IfThenElse};

impl EffectBehaviors for IfThenElse {
    fn needs_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
    ) -> usize {
        if source.passes_restrictions(db, LogId::current(db), source, &self.if_) {
            self.then
                .iter()
                .map(|effect| effect.effect.as_ref().unwrap().needs_targets(db, source))
                .max()
                .unwrap()
        } else {
            self.else_
                .iter()
                .map(|effect| effect.effect.as_ref().unwrap().needs_targets(db, source))
                .max()
                .unwrap_or_default()
        }
    }

    fn wants_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
    ) -> usize {
        if source.passes_restrictions(db, LogId::current(db), source, &self.if_) {
            self.then
                .iter()
                .map(|effect| effect.effect.as_ref().unwrap().wants_targets(db, source))
                .max()
                .unwrap()
        } else {
            self.else_
                .iter()
                .map(|effect| effect.effect.as_ref().unwrap().wants_targets(db, source))
                .max()
                .unwrap_or_default()
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
            for effect in self.then.iter() {
                effect
                    .effect
                    .as_ref()
                    .unwrap()
                    .push_pending_behavior(db, source, controller, results);
            }
        } else {
            for effect in self.else_.iter() {
                effect
                    .effect
                    .as_ref()
                    .unwrap()
                    .push_pending_behavior(db, source, controller, results);
            }
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
            for effect in self.then.iter() {
                effect.effect.as_ref().unwrap().push_behavior_with_targets(
                    db,
                    targets.clone(),
                    apply_to_self,
                    source,
                    controller,
                    results,
                );
            }
        } else {
            for effect in self.else_.iter() {
                effect.effect.as_ref().unwrap().push_behavior_with_targets(
                    db,
                    targets.clone(),
                    apply_to_self,
                    source,
                    controller,
                    results,
                );
            }
        }
    }
}
