use std::collections::HashSet;

use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::{ActionResult, ChooseTargets, TargetSource},
    controller::ControllerRestriction,
    effects::{Effect, EffectBehaviors},
    in_play::{self, OnBattlefield},
    stack::ActiveTarget,
    types::Type,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyOfAnyCreatureNonTargeting;

impl EffectBehaviors for CopyOfAnyCreatureNonTargeting {
    fn needs_targets(&self) -> usize {
        1
    }

    fn wants_targets(&self) -> usize {
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
            card.passes_restrictions(
                db,
                source,
                ControllerRestriction::Any,
                &source.restrictions(db),
            ) && card.is_in_location::<OnBattlefield>(db)
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
        ));
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::CloneCreatureNonTargeting {
            source,
            target: Some(targets.into_iter().exactly_one().unwrap()),
        })
    }
}
