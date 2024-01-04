use crate::{
    battlefield::ActionResult, effects::EffectBehaviors, player::Owner, protogen,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct ControllerDiscards {
    count: usize,
    unless: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::ControllerDiscards> for ControllerDiscards {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ControllerDiscards) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
            unless: value
                .unless
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for ControllerDiscards {
    fn needs_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if self.unless.is_empty()
            || !Owner::from(controller).passes_restrictions(db, controller, &self.unless)
        {
            results.push_settled(ActionResult::DiscardCards {
                target: controller,
                count: self.count,
            });
        }
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if self.unless.is_empty()
            || !Owner::from(controller).passes_restrictions(db, controller, &self.unless)
        {
            results.push_settled(ActionResult::DiscardCards {
                target: controller,
                count: self.count,
            });
        }
    }
}
