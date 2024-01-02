use crate::{battlefield::ActionResult, effects::EffectBehaviors, protogen};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Scry {
    count: usize,
}

impl TryFrom<&protogen::effects::Scry> for Scry {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Scry) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
        })
    }
}

impl EffectBehaviors for Scry {
    fn needs_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Scry(source, self.count));
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Scry(source, self.count));
    }
}
