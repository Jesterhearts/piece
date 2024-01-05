use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors, ModifyBattlefield},
    in_play::{self, target_from_location},
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen,
};

#[derive(Debug, Clone)]
pub(crate) struct CreateTokenCopy {
    modifiers: Vec<ModifyBattlefield>,
}

impl TryFrom<&protogen::effects::CreateTokenCopy> for CreateTokenCopy {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::CreateTokenCopy) -> Result<Self, Self::Error> {
        Ok(Self {
            modifiers: value
                .modifiers
                .iter()
                .map(ModifyBattlefield::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for CreateTokenCopy {
    fn needs_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
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
        for target in in_play::all_cards(db)
            .into_iter()
            .filter(|card| card.passes_restrictions(db, source, &source.restrictions(db)))
            .collect_vec()
        {
            if target.can_be_targeted(db, controller) {
                let target = target_from_location(db, target);
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
            }
        }

        targets
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());
        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
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
        results: &mut crate::pending_results::PendingResults,
    ) {
        let target = targets.into_iter().exactly_one().unwrap();
        let target = target.id();
        results.push_settled(ActionResult::CreateTokenCopyOf {
            source,
            target: target.unwrap(),
            modifiers: self.modifiers.clone(),
        });
    }

    fn push_behavior_from_top_of_library(
        &self,
        _db: &in_play::Database,
        source: in_play::CardId,
        target: in_play::CardId,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::CreateTokenCopyOf {
            source,
            target,
            modifiers: self.modifiers.clone(),
        })
    }
}
