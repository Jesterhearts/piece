use itertools::Itertools;

use crate::{
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    log::LogId,
    pending_results::{choose_targets::ChooseTargets, PendingResults, TargetSource},
    player::Controller,
    protogen::effects::{effect::Effect, ChooseCast},
    stack::ActiveTarget,
};

impl EffectBehaviors for ChooseCast {
    fn needs_targets(&self, _db: &Database, _source: CardId) -> usize {
        1
    }

    fn wants_targets(&self, _db: &Database, _source: CardId) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &Database,
        source: CardId,
        log_session: LogId,
        _controller: Controller,
        _already_chosen: &std::collections::HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        db.cards
            .keys()
            .copied()
            .filter(|card| {
                card.passes_restrictions(
                    db,
                    log_session,
                    source,
                    &source.faceup_face(db).restrictions,
                ) && card.passes_restrictions(db, log_session, source, &self.restrictions)
            })
            .filter_map(|card| card.target_from_location(db))
            .collect_vec()
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    ) {
        let targets = self.valid_targets(
            db,
            source,
            LogId::current(db),
            controller,
            results.all_currently_targeted(),
        );

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            targets,
            LogId::current(db),
            source,
        ))
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        _apply_to_self: bool,
        _source: CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        for target in targets {
            results.push_choose_cast(target.id(db).unwrap(), false, false);
        }
    }
}