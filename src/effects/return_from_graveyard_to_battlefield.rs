use itertools::Itertools;

use crate::{
    battlefield::{compute_graveyard_targets, ActionResult},
    effects::{Effect, EffectBehaviors},
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct ReturnFromGraveyardToBattlefield {
    pub(crate) count: usize,
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::ReturnFromGraveyardToBattlefield>
    for ReturnFromGraveyardToBattlefield
{
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::ReturnFromGraveyardToBattlefield,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for ReturnFromGraveyardToBattlefield {
    fn needs_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        self.count
    }

    fn wants_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        self.count
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        compute_graveyard_targets(db, source, &self.restrictions)
            .into_iter()
            .map(|card| ActiveTarget::Graveyard { id: card })
            .collect_vec()
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
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::ReturnFromGraveyardToBattlefield { targets });
    }
}
