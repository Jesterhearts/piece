use crate::{battlefield::ActionResult, effects::EffectBehaviors, protogen};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Discover {
    count: usize,
}

impl TryFrom<&protogen::effects::Discover> for Discover {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Discover) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
        })
    }
}

impl EffectBehaviors for Discover {
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
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Discover {
            source,
            count: self.count,
            player: controller,
        })
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Discover {
            source,
            count: self.count,
            player: controller,
        })
    }
}
