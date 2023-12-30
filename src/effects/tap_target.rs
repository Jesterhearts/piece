use itertools::Itertools;
use tracing::Level;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult},
    effects::{Effect, EffectBehaviors},
    in_play::{self, OnBattlefield},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug)]
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
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn valid_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        in_play::cards::<OnBattlefield>(db)
            .into_iter()
            .filter(|card| {
                card.passes_restrictions(
                    db,
                    source,
                    crate::controller::ControllerRestriction::Any,
                    &self.restrictions,
                ) && card.passes_restrictions(
                    db,
                    source,
                    crate::controller::ControllerRestriction::Any,
                    &source.restrictions(db),
                )
            })
            .map(|card| crate::stack::ActiveTarget::Battlefield { id: card })
            .filter(|target| !already_chosen.contains(target))
            .collect_vec()
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());

        results.push_choose_targets(ChooseTargets::new(
            crate::battlefield::TargetSource::Effect(Effect(self)),
            valid_targets,
            source,
        ));
    }

    #[instrument(level = Level::INFO, skip(_db, results))]
    fn push_behavior_with_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if let Ok(ActiveTarget::Battlefield { id }) = targets.into_iter().exactly_one() {
            results.push_settled(ActionResult::TapPermanent(id))
        } else {
            warn!("Skipping targeting")
        }
    }
}
