use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::{self},
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::effects::{effect::Effect, CreateTokenCopy},
};

impl EffectBehaviors for CreateTokenCopy {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: &crate::protogen::ids::CardId,
        log_session: crate::log::LogId,
        controller: &crate::protogen::ids::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for target in db.cards.keys().filter(|card| {
            card.passes_restrictions(
                db,
                log_session,
                source,
                &source.faceup_face(db).restrictions,
            )
        }) {
            if target.can_be_targeted(db, controller) {
                let target = target.target_from_location(db).unwrap();
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
            }
        }

        targets
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
        _controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let target = targets.into_iter().exactly_one().unwrap();
        let target = target.id(db).unwrap();
        results.push_settled(ActionResult::CreateTokenCopyOf {
            source: source.clone(),
            target: target.clone(),
            modifiers: self.modifiers.clone(),
        });
    }

    fn push_behavior_from_top_of_library(
        &self,
        _db: &in_play::Database,
        source: &crate::protogen::ids::CardId,
        target: crate::protogen::ids::CardId,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::CreateTokenCopyOf {
            source: source.clone(),
            target,
            modifiers: self.modifiers.clone(),
        })
    }
}
