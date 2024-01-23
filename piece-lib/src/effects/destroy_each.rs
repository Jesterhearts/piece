use crate::{
    action_result::{self, ActionResult},
    effects::EffectBehaviors,
    protogen::effects::DestroyEach,
};

impl EffectBehaviors for DestroyEach {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::from(
            action_result::destroy_each::DestroyEach {
                source,
                restrictions: self.restrictions.clone(),
            },
        ));
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::from(
            action_result::destroy_each::DestroyEach {
                source,
                restrictions: self.restrictions.clone(),
            },
        ));
    }
}
