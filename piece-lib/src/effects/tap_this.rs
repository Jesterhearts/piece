use crate::{
    action_result::{tap_permanent::TapPermanent, ActionResult},
    effects::EffectBehaviors,
    protogen::effects::TapThis,
};

impl EffectBehaviors for TapThis {
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
        results.push_settled(ActionResult::from(TapPermanent { card: source }));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
