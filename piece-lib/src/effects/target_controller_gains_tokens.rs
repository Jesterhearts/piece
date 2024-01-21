use itertools::Itertools;

use crate::{
    action_result::ActionResult, effects::EffectBehaviors,
    protogen::effects::TargetControllerGainsTokens,
};

impl EffectBehaviors for TargetControllerGainsTokens {
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
        _controller: &crate::protogen::ids::Controller,
        _results: &mut crate::pending_results::PendingResults,
    ) {
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _source: &crate::protogen::ids::CardId,
        _controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::CreateToken {
            source: targets
                .into_iter()
                .exactly_one()
                .unwrap()
                .id(db)
                .unwrap()
                .clone(),
            token: self.create_token.token.as_ref().unwrap().clone(),
        });
    }
}
