use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::{
        choose_targets::ChooseTargets, compute_graveyard_targets, ActionResult, TargetSource,
    },
    controller::ControllerRestriction,
    effects::{Effect, EffectBehaviors},
    protogen,
    stack::ActiveTarget,
    types::Type,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnFromGraveyardToBattlefield {
    pub count: usize,
    pub types: IndexSet<Type>,
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
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for ReturnFromGraveyardToBattlefield {
    fn needs_targets(&self) -> usize {
        self.count
    }

    fn wants_targets(&self) -> usize {
        self.count
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        compute_graveyard_targets(db, ControllerRestriction::You, source, &self.types)
            .into_iter()
            .map(|card| ActiveTarget::Graveyard { id: card })
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

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::ReturnFromGraveyardToBattlefield { targets });
    }
}
