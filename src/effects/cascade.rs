use crate::{battlefield::ActionResult, effects::EffectBehaviors};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cascade;

impl EffectBehaviors for Cascade {
    fn needs_targets(&self) -> usize {
        0
    }

    fn wants_targets(&self) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Cascade {
            cascading: source.cost(db).cmc(),
            player: controller,
        });
    }

    fn push_behavior_from_top_of_library(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        _target: crate::in_play::CardId,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Cascade {
            cascading: source.cost(db).cmc(),
            player: source.controller(db),
        })
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Cascade {
            cascading: source.cost(db).cmc(),
            player: controller,
        })
    }
}
