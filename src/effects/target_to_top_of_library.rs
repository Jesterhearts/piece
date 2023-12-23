use std::sync::Arc;

use itertools::Itertools;

use crate::{
    battlefield::{ActionResult, ChooseTargets, TargetSource},
    controller::ControllerRestriction,
    effects::{Effect, EffectBehaviors},
    in_play::{self, OnBattlefield},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub struct TargetToTopOfLibrary {
    restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::TargetToTopOfLibrary> for TargetToTopOfLibrary {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::TargetToTopOfLibrary) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for TargetToTopOfLibrary {
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
        for target in in_play::cards::<OnBattlefield>(db)
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
            if target.can_be_targeted(db, controller)
                && target.passes_restrictions(
                    db,
                    source,
                    ControllerRestriction::Any,
                    &self.restrictions,
                )
            {
                let target = ActiveTarget::Battlefield { id: target };
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
        results: &mut crate::battlefield::PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());
        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect(Arc::new(self.clone()) as Arc<_>)),
            valid_targets,
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
        for target in targets {
            results.push_settled(ActionResult::ReturnFromBattlefieldToLibrary { target });
        }
    }
}
