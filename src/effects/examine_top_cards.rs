use indexmap::IndexMap;

use crate::{
    battlefield::ActionResult,
    effects::{Destination, EffectBehaviors},
    protogen,
};

#[derive(Debug, Clone)]
pub(crate) struct ExamineTopCards {
    count: usize,
    destinations: IndexMap<Destination, usize>,
}

impl TryFrom<&protogen::effects::ExamineTopCards> for ExamineTopCards {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ExamineTopCards) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
            destinations: value
                .destinations
                .iter()
                .map(|dest| -> anyhow::Result<_> {
                    Ok((
                        Destination::try_from(dest.destination.get_or_default())?,
                        usize::try_from(dest.count)?,
                    ))
                })
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for ExamineTopCards {
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
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::ExamineTopCards {
            destinations: self.destinations.clone(),
            count: self.count,
            controller,
        });
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
