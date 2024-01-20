use crate::{
    action_result::ActionResult, effects::EffectBehaviors, protogen::effects::ExamineTopCards,
};

impl EffectBehaviors for ExamineTopCards {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::ExamineTopCards {
            destinations: self.destinations.clone(),
            count: self.count,
            controller,
        });
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        source: &crate::protogen::ids::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
