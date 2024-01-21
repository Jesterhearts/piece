use itertools::Itertools;

use crate::{
    action_result::ActionResult, effects::EffectBehaviors, protogen::effects::CantAttackThisTurn,
    stack::ActiveTarget,
};

impl EffectBehaviors for CantAttackThisTurn {
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

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: &crate::protogen::ids::CardId,
        log_session: crate::log::LogId,
        controller: &crate::protogen::ids::Controller,
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        db.all_players
            .all_players()
            .into_iter()
            .filter(|player| {
                player.passes_restrictions(
                    db,
                    log_session,
                    controller,
                    &source.faceup_face(db).restrictions,
                ) && player.passes_restrictions(db, log_session, controller, &self.restrictions)
            })
            .map(|player| ActiveTarget::Player { id: player })
            .collect_vec()
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
        _controller: &crate::protogen::ids::Controller,
        _results: &mut crate::pending_results::PendingResults,
    ) {
        unreachable!()
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _source: &crate::protogen::ids::CardId,
        _controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        for target in targets {
            let ActiveTarget::Player { id } = target else {
                warn!("Skipping target {:?}", target);
                continue;
            };

            results.push_settled(ActionResult::BanAttacking(id));
        }
    }
}
