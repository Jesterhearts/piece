use crate::{action_result::ActionResult, effects::EffectBehaviors, protogen::effects::Cascade};

impl EffectBehaviors for Cascade {
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
        source: &crate::protogen::ids::CardId,
        controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::Cascade {
            source: source.clone(),
            cascading: source.faceup_face(db).cost.cmc(),
            player: controller.clone(),
        });
    }

    fn push_behavior_from_top_of_library(
        &self,
        db: &crate::in_play::Database,
        source: &crate::protogen::ids::CardId,
        _target: crate::protogen::ids::CardId,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::Cascade {
            source: source.clone(),
            cascading: source.faceup_face(db).cost.cmc(),
            player: db[source].controller.clone(),
        })
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        source: &crate::protogen::ids::CardId,
        controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::Cascade {
            source: source.clone(),
            cascading: source.faceup_face(db).cost.cmc(),
            player: controller.clone(),
        })
    }
}
