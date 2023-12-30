use itertools::Itertools;
use tracing::Level;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    effects::{Effect, EffectBehaviors},
    in_play::{all_cards, target_from_location},
    protogen,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct ReturnTargetToHand {
    restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::ReturnTargetToHand> for ReturnTargetToHand {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ReturnTargetToHand) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for ReturnTargetToHand {
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
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        all_cards(db)
            .into_iter()
            .filter_map(|card| {
                if card.passes_restrictions(db, source, &self.restrictions)
                    && card.passes_restrictions(db, source, &source.restrictions(db))
                    && card.can_be_targeted(db, controller)
                {
                    Some(target_from_location(db, card))
                } else {
                    None
                }
            })
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
            TargetSource::Effect(Effect(self)),
            valid_targets,
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
        results: &mut crate::battlefield::PendingResults,
    ) {
        if let Ok(Some(target)) = targets.into_iter().exactly_one().map(|t| t.id()) {
            results.push_settled(ActionResult::HandFromBattlefield(target))
        } else {
            warn!("Skipping targets")
        }
    }
}
