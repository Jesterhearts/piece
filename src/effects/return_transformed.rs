use crate::{battlefield::ActionResult, effects::EffectBehaviors, protogen};

#[derive(Debug, Clone)]
pub(crate) struct ReturnTransformed {
    enters_tapped: bool,
}

impl TryFrom<&protogen::effects::ReturnTransformed> for ReturnTransformed {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ReturnTransformed) -> Result<Self, Self::Error> {
        Ok(Self {
            enters_tapped: value.enters_tapped,
        })
    }
}

impl EffectBehaviors for ReturnTransformed {
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
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::ReturnTransformed {
            target: source,
            enters_tapped: self.enters_tapped,
        })
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
        results.push_settled(ActionResult::ReturnTransformed {
            target: source,
            enters_tapped: self.enters_tapped,
        })
    }
}
