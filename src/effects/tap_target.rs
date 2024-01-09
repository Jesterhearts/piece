use itertools::Itertools;
use tracing::Level;

use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors},
    pending_results::choose_targets::ChooseTargets,
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct TapTarget {
    restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::TapTarget> for TapTarget {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::TapTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for TapTarget {
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
        db.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .filter(|card| {
                card.passes_restrictions(db, log_session, source, &self.restrictions)
                    && card.passes_restrictions(
                        db,
                        log_session,
                        source,
                        &source.faceup_face(db).restrictions,
                    )
            })
            .map(|card| crate::stack::ActiveTarget::Battlefield { id: *card })
            .filter(|target| !already_chosen.contains(target))
            .collect_vec()
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
            crate::pending_results::TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            crate::log::LogId::current(db),
            source,
        ));
    }

    #[instrument(level = Level::INFO, skip(_db, results))]
    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if let Ok(ActiveTarget::Battlefield { id }) = targets.into_iter().exactly_one() {
            results.push_settled(ActionResult::TapPermanent(id))
        } else {
            warn!("Skipping targeting")
        }
    }
}
