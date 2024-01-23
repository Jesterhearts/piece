use crate::{
    action_result::{self, ActionResult},
    effects::EffectBehaviors,
    protogen::effects::GainLife,
};

impl EffectBehaviors for GainLife {
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
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::from(action_result::gain_life::GainLife {
            target: controller,
            count: self.count,
        }));
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::from(action_result::gain_life::GainLife {
            target: controller,
            count: self.count,
        }));
    }
}
