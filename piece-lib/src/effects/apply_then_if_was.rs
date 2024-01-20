use crate::{
    action_result::ActionResult, effects::EffectBehaviors, log::LogId,
    protogen::effects::ApplyThenIfWas,
};

impl EffectBehaviors for ApplyThenIfWas {
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

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        _log_session: crate::log::LogId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        self.apply
            .iter()
            .map(|effect| {
                effect.effect.as_ref().unwrap().valid_targets(
                    db,
                    source,
                    LogId::current(db),
                    controller,
                    already_chosen,
                )
            })
            .max_by_key(|targets| targets.len())
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
        results.push_settled(ActionResult::IfWasThen {
            if_was: self.then.if_was.clone(),
            then: self.then.apply.clone(),
            source,
            controller,
        })
    }
}
