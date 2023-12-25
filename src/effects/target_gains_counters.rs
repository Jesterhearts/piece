use itertools::Itertools;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    controller::ControllerRestriction,
    effects::{gain_counter::GainCounter, Effect, EffectBehaviors},
    in_play::{self, target_from_location},
    protogen,
    stack::ActiveTarget,
};

#[derive(Debug, Clone, Copy)]
pub struct TargetGainsCounters {
    gain: GainCounter,
}

impl TryFrom<&protogen::effects::GainCounter> for TargetGainsCounters {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::GainCounter) -> Result<Self, Self::Error> {
        Ok(Self {
            gain: value.try_into()?,
        })
    }
}

impl EffectBehaviors for TargetGainsCounters {
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
        for card in in_play::all_cards(db) {
            if card.passes_restrictions(
                db,
                source,
                ControllerRestriction::Any,
                &source.restrictions(db),
            ) && card.can_be_targeted(db, controller)
            {
                let target = target_from_location(db, card);
                if already_chosen.contains(&target) {
                    continue;
                }
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
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let target = match targets.into_iter().exactly_one().unwrap() {
            ActiveTarget::Battlefield { id } => id,
            ActiveTarget::Graveyard { id } => id,
            _ => unreachable!(),
        };

        results.push_settled(ActionResult::AddCounters {
            source,
            target,
            counter: self.gain,
        })
    }
}
