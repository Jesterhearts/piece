use crate::{battlefield::ActionResult, effects::EffectBehaviors};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Cascade;

impl EffectBehaviors for Cascade {
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
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::Cascade {
            source,
            cascading: source.faceup_face(db).cost.cmc(),
            player: controller,
        });
    }

    fn push_behavior_from_top_of_library(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        _target: crate::in_play::CardId,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::Cascade {
            source,
            cascading: source.faceup_face(db).cost.cmc(),
            player: db[source].controller,
        })
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::Cascade {
            source,
            cascading: source.faceup_face(db).cost.cmc(),
            player: controller,
        })
    }
}
