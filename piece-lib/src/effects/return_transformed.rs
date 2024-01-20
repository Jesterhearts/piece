use crate::{
    action_result::ActionResult, effects::EffectBehaviors, protogen::effects::ReturnTransformed,
};

impl EffectBehaviors for ReturnTransformed {
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
        source: &crate::protogen::ids::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::ReturnTransformed {
            target: source.clone(),
            enters_tapped: self.enters_tapped,
        })
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        source: &crate::protogen::ids::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::ReturnTransformed {
            target: source.clone(),
            enters_tapped: self.enters_tapped,
        })
    }
}
