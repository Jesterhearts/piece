use std::vec::IntoIter;

use crate::{
    action_result::ActionResult,
    effects::{EffectBehaviors, ReplacementEffect},
    log::LogId,
    pending_results::PendingResults,
    player::Player,
    protogen::{effects::ControllerLosesLife, ids::Owner},
};

impl EffectBehaviors for ControllerLosesLife {
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
            results.push_settled(ActionResult::LoseLife {
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
            results.push_settled(ActionResult::LoseLife {
                target: controller.clone(),
                count: self.count,
            });
        }
    }

    fn replace_draw(
        &self,
        db: &mut crate::in_play::Database,
        player: &crate::protogen::ids::Owner,
        replacements: &mut IntoIter<(crate::protogen::ids::CardId, ReplacementEffect)>,
        controller: &crate::protogen::ids::Controller,
        count: usize,
        results: &mut PendingResults,
    ) {
        if self.unless.is_empty()
            || !Owner::from(controller.clone()).passes_restrictions(
                db,
                LogId::current(db),
                controller,
                &self.unless,
            )
        {
            results.push_settled(ActionResult::LoseLife {
                target: controller.clone(),
                count: self.count,
            });
        }

        Player::draw_with_replacement(db, player, replacements, count, results);
    }
}
