use itertools::Itertools;
use tracing::Level;

use crate::{
    action_result::ActionResult,
    effects::{Effect, EffectBehaviors},
    in_play::target_from_location,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::{self, targets::Restriction},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReturnTargetToHand {
    restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::ReturnTargetToHand> for ReturnTargetToHand {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ReturnTargetToHand) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value.restrictions.clone(),
        })
    }
}

impl EffectBehaviors for ReturnTargetToHand {
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
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        db.cards
            .keys()
            .filter_map(|card| {
                if card.passes_restrictions(db, log_session, source, &self.restrictions)
                    && card.passes_restrictions(
                        db,
                        log_session,
                        source,
                        &source.faceup_face(db).restrictions,
                    )
                    && card.can_be_targeted(db, controller)
                {
                    Some(target_from_location(db, *card))
                } else {
                    None
                }
            })
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
            TargetSource::Effect(Effect::from(self.clone())),
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
        if let Ok(Some(target)) = targets.into_iter().exactly_one().map(|t| t.id()) {
            results.push_settled(ActionResult::HandFromBattlefield(target))
        } else {
            warn!("Skipping targets")
        }
    }
}