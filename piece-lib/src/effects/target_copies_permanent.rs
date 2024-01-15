use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    log::LogId,
    pending_results::{choose_targets::ChooseTargets, PendingResults, TargetSource},
    player::Controller,
    protogen::{
        effects::{effect::Effect, TargetCopiesPermanent},
        targets::Location,
    },
    stack::ActiveTarget,
};

impl EffectBehaviors for TargetCopiesPermanent {
    fn needs_targets(&self, _db: &Database, _source: CardId) -> usize {
        2
    }

    fn wants_targets(&self, _db: &Database, _source: CardId) -> usize {
        2
    }

    fn valid_targets(
        &self,
        db: &Database,
        source: CardId,
        log_session: LogId,
        _controller: Controller,
        already_chosen: &std::collections::HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        if already_chosen.is_empty() {
            db.cards
                .keys()
                .filter(|card| {
                    card.is_permanent(db) && card.is_in_location(db, Location::ON_BATTLEFIELD)
                })
                .filter(|card| {
                    card.passes_restrictions(
                        db,
                        log_session,
                        source,
                        &card.faceup_face(db).restrictions,
                    ) && card.passes_restrictions(
                        db,
                        log_session,
                        source,
                        &self.target_restrictions,
                    )
                })
                .copied()
                .map(|card| card.target_from_location(db))
                .collect_vec()
        } else {
            db.cards
                .keys()
                .filter(|card| card.is_permanent(db))
                .filter(|card| {
                    card.passes_restrictions(
                        db,
                        log_session,
                        source,
                        &card.faceup_face(db).restrictions,
                    ) && card.passes_restrictions(db, log_session, source, &self.copy_restrictions)
                })
                .copied()
                .map(|card| card.target_from_location(db))
                .collect_vec()
        }
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    ) {
        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            self.valid_targets(
                db,
                source,
                LogId::current(db),
                controller,
                results.all_currently_targeted(),
            ),
            LogId::current(db),
            source,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        mut targets: Vec<ActiveTarget>,
        _apply_to_self: bool,
        _source: CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        assert_eq!(targets.len(), 2);

        let cloned = targets.pop().unwrap().id(db).unwrap();
        let Some(ActiveTarget::Battlefield { id }) = targets.pop() else {
            unreachable!()
        };

        results.push_settled(ActionResult::CloneCard {
            cloning: id,
            cloned,
        });
    }
}
