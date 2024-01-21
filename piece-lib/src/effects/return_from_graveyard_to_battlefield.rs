use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    log::LogId,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::effects::{effect::Effect, ReturnFromGraveyardToBattlefield},
    stack::ActiveTarget,
};

impl EffectBehaviors for ReturnFromGraveyardToBattlefield {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        self.count as usize
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        self.count as usize
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: &crate::protogen::ids::CardId,
        log_session: crate::log::LogId,
        _controller: &crate::protogen::ids::Controller,
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        db.graveyard
            .graveyards
            .values()
            .flat_map(|g| g.iter())
            .filter(|card| {
                card.passes_restrictions(
                    db,
                    log_session,
                    source,
                    &source.faceup_face(db).restrictions,
                ) && card.passes_restrictions(db, log_session, source, &self.restrictions)
            })
            .map(|card| ActiveTarget::Graveyard { id: card.clone() })
            .collect_vec()
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: &crate::protogen::ids::CardId,
        controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let valid_targets = self.valid_targets(
            db,
            source,
            crate::log::LogId::current(db),
            controller,
            results.all_currently_targeted(),
        );

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            crate::log::LogId::current(db),
            source.clone(),
        ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        source: &crate::protogen::ids::CardId,
        controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let valid = self
            .valid_targets(
                db,
                source,
                LogId::current(db),
                controller,
                &HashSet::default(),
            )
            .into_iter()
            .collect::<HashSet<_>>();

        for target in targets {
            if valid.contains(&target) {
                results.push_settled(ActionResult::ReturnFromGraveyardToBattlefield { target });
            }
        }
    }
}
