use itertools::Itertools;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    controller::ControllerRestriction,
    effects::{Effect, EffectBehaviors},
    in_play::{self, target_from_location, OnBattlefield},
    stack::ActiveTarget,
};

#[derive(Debug, Clone, Copy)]
pub struct UntapTarget;

impl EffectBehaviors for UntapTarget {
    fn needs_targets(&self) -> usize {
        1
    }

    fn wants_targets(&self) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in in_play::cards::<OnBattlefield>(db)
            .into_iter()
            .filter(|card| {
                card.passes_restrictions(
                    db,
                    source,
                    ControllerRestriction::Any,
                    &source.restrictions(db),
                )
            })
            .collect_vec()
        {
            let target = target_from_location(db, card);
            if card.can_be_targeted(db, controller) && !already_chosen.contains(&target) {
                targets.push(target);
            }
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

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let Ok(ActiveTarget::Battlefield { id }) = targets.into_iter().exactly_one() else {
            unreachable!()
        };
        results.push_settled(ActionResult::Untap(id));
    }
}
