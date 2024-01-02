use std::collections::HashSet;

use indexmap::IndexSet;
use itertools::Itertools;
use tracing::Level;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    effects::{Effect, EffectBehaviors},
    in_play::{self, OnBattlefield},
    stack::ActiveTarget,
    types::Type,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CopyOfAnyCreatureNonTargeting;

impl EffectBehaviors for CopyOfAnyCreatureNonTargeting {
    fn needs_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: in_play::CardId,
        _controller: crate::player::Controller,
        already_chosen: &HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for creature in in_play::all_cards(db).into_iter().filter(|card| {
            card.passes_restrictions(db, source, &source.restrictions(db))
                && card.is_in_location::<OnBattlefield>(db)
                && card.types_intersect(db, &IndexSet::from([Type::Creature]))
        }) {
            let target = ActiveTarget::Battlefield { id: creature };
            if already_chosen.contains(&target) {
                continue;
            }
            targets.push(target);
        }

        targets
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
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if let Ok(target) = targets.into_iter().exactly_one() {
            results.push_settled(ActionResult::CloneCreatureNonTargeting { source, target })
        } else {
            warn!("Skipping targets");
        }
    }
}
