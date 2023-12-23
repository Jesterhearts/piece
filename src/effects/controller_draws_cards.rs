use std::vec::IntoIter;

use crate::{
    battlefield::{ActionResult, PendingResults},
    effects::EffectBehaviors,
    in_play::{Database, ReplacementEffectId},
    player::Player,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ControllerDrawsCards {
    pub count: usize,
}

impl EffectBehaviors for ControllerDrawsCards {
    fn needs_targets(&self) -> usize {
        0
    }

    fn wants_targets(&self) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::DrawCards {
            target: controller,
            count: self.count,
        });
    }

    fn push_behavior_from_top_of_library(
        &self,
        db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
        target: crate::in_play::CardId,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::DrawCards {
            target: target.controller(db),
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
        results.push_settled(ActionResult::DrawCards {
            target: controller,
            count: self.count,
        });
    }

    fn replace_draw(
        &self,
        player: &mut Player,
        db: &mut Database,
        replacements: &mut IntoIter<ReplacementEffectId>,
        _controller: crate::player::Controller,
        _count: usize,
        results: &mut PendingResults,
    ) {
        player.draw_with_replacement(db, replacements, self.count, results);
    }
}
