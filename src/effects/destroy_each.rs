use crate::{battlefield::ActionResult, effects::EffectBehaviors, protogen, targets::Restriction};

#[derive(Debug, Clone)]
pub(crate) struct DestroyEach {
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::DestroyEach> for DestroyEach {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::DestroyEach) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for DestroyEach {
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        0
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::DestroyEach(source, self.restrictions.clone()));
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
        results.push_settled(ActionResult::DestroyEach(source, self.restrictions.clone()));
    }
}
