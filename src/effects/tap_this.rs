use crate::{battlefield::ActionResult, effects::EffectBehaviors};

#[derive(Debug)]
pub(crate) struct TapThis;

impl EffectBehaviors for TapThis {
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        0
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        0
    }

    fn push_pending_behavior(
        &'static self,
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::TapPermanent(source));
    }

    fn push_behavior_with_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
