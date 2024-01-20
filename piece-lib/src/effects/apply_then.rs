use crate::{action_result::ActionResult, effects::EffectBehaviors, protogen::effects::ApplyThen};

impl EffectBehaviors for ApplyThen {
    fn needs_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
    ) -> usize {
        self.apply
            .iter()
            .map(|effect| effect.effect.as_ref().unwrap().needs_targets(db, source))
            .max()
            .unwrap()
    }

    fn wants_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
    ) -> usize {
        self.apply
            .iter()
            .map(|effect| effect.effect.as_ref().unwrap().wants_targets(db, source))
            .max()
            .unwrap()
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        for effect in self.apply.iter() {
            effect
                .effect
                .as_ref()
                .unwrap()
                .push_pending_behavior(db, source, controller, results);
        }

        results.push_settled(ActionResult::ThenApply {
            apply: self.then.clone(),
            source,
            controller,
        })
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        for effect in self.apply.iter() {
            effect.effect.as_ref().unwrap().push_behavior_with_targets(
                db,
                targets.clone(),
                source,
                controller,
                results,
            );
        }

        results.push_settled(ActionResult::ThenApply {
            apply: self.then.clone(),
            source,
            controller,
        });
    }
}
