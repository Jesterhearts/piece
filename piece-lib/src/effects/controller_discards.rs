use crate::{
    action_result::ActionResult, effects::EffectBehaviors, log::LogId,
    protogen::effects::ControllerDiscards, protogen::ids::Owner,
};

impl EffectBehaviors for ControllerDiscards {
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
        db: &mut crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
        controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if self.unless.is_empty()
            || !Owner::from(controller.clone()).passes_restrictions(
                db,
                LogId::current(db),
                controller,
                &self.unless,
            )
        {
            results.push_settled(ActionResult::DiscardCards {
                target: controller.clone(),
                count: self.count,
            });
        }
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _source: &crate::protogen::ids::CardId,
        controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if self.unless.is_empty()
            || !Owner::from(controller.clone()).passes_restrictions(
                db,
                LogId::current(db),
                controller,
                &self.unless,
            )
        {
            results.push_settled(ActionResult::DiscardCards {
                target: controller.clone(),
                count: self.count,
            });
        }
    }
}
