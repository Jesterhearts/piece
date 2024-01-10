use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors, EffectDuration},
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::types::type_::TypeDiscriminants,
    stack::ActiveTarget,
    types::TypeSet,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub(crate) struct ExileTargetCreatureManifestTopOfLibrary;

impl EffectBehaviors for ExileTargetCreatureManifestTopOfLibrary {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        _controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in db.battlefield.battlefields.values().flat_map(|b| b.iter()) {
            if card.passes_restrictions(
                db,
                log_session,
                source,
                &source.faceup_face(db).restrictions,
            ) && card.types_intersect(db, &TypeSet::from([TypeDiscriminants::Creature]))
            {
                let target = ActiveTarget::Battlefield { id: *card };
                if already_chosen.contains(&target) {
                    continue;
                }
                targets.push(target);
            }
        }

        targets
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
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
            TargetSource::Effect(Effect::from(*self)),
            valid_targets,
            crate::log::LogId::current(db),
            source,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        for target in targets {
            results.push_settled(ActionResult::ExileTarget {
                source,
                target,
                duration: EffectDuration::Permanently,
                reason: None,
            });
            results.push_settled(ActionResult::ManifestTopOfLibrary(
                db[target.id().unwrap()].controller,
            ));
        }
    }
}
