use std::vec::IntoIter;

use crate::{
    battlefield::{ActionResult, PendingResults},
    effects::EffectBehaviors,
    in_play::ReplacementEffectId,
    player::{Owner, Player},
    protogen,
    targets::Restriction,
};

#[derive(Debug)]
pub(crate) struct ControllerLosesLife {
    count: usize,
    unless: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::ControllerLosesLife> for ControllerLosesLife {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ControllerLosesLife) -> Result<Self, Self::Error> {
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

impl EffectBehaviors for ControllerLosesLife {
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        0
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
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
            results.push_settled(ActionResult::LoseLife {
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
            results.push_settled(ActionResult::LoseLife {
                target: controller,
                count: self.count,
            });
        }
    }

    fn replace_draw(
        &self,
        _player: &mut Player,
        db: &mut crate::in_play::Database,
        _replacements: &mut IntoIter<ReplacementEffectId>,
        controller: crate::player::Controller,
        _count: usize,
        results: &mut PendingResults,
    ) {
        if self.unless.is_empty()
            || !Owner::from(controller).passes_restrictions(db, controller, &self.unless)
        {
            results.push_settled(ActionResult::LoseLife {
                target: controller,
                count: self.count,
            });
        }
    }
}
