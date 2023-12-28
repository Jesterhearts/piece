use crate::{battlefield::ActionResult, effects::EffectBehaviors};

#[derive(Debug)]
pub(crate) struct Transform;

impl EffectBehaviors for Transform {
    fn needs_targets(&'static self) -> usize {
        0
    }

    fn wants_targets(&'static self) -> usize {
        0
    }

    fn push_pending_behavior(
        &'static self,
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Transform { target: source })
    }

    fn push_behavior_with_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Transform { target: source })
    }
}
