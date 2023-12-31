use crate::{battlefield::ActionResult, effects::EffectBehaviors};

#[derive(Debug)]
pub(crate) struct ControllerDiscards {
    pub(crate) count: usize,
}

impl EffectBehaviors for ControllerDiscards {
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        0
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::DiscardCards {
            target: controller,
            count: self.count,
        });
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::DiscardCards {
            target: controller,
            count: self.count,
        });
    }
}
