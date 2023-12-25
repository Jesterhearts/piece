use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    controller::ControllerRestriction,
    effects::{Effect, EffectBehaviors},
    player::AllPlayers,
    protogen,
    stack::ActiveTarget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mill {
    count: usize,
    target: ControllerRestriction,
}

impl TryFrom<&protogen::effects::Mill> for Mill {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Mill) -> Result<Self, Self::Error> {
        Ok(Self {
            count: usize::try_from(value.count)?,
            target: value.target.get_or_default().try_into()?,
        })
    }
}

impl EffectBehaviors for Mill {
    fn needs_targets(&self) -> usize {
        1
    }

    fn wants_targets(&self) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        _already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        match self.target {
            ControllerRestriction::Any => AllPlayers::all_players_in_db(db),
            ControllerRestriction::You => HashSet::from([controller.into()]),
            ControllerRestriction::Opponent => {
                let mut all = AllPlayers::all_players_in_db(db);
                all.remove(&controller.into());
                all
            }
        }
        .into_iter()
        .map(|player| ActiveTarget::Player { id: player })
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
        results.push_settled(ActionResult::Mill {
            count: self.count,
            targets,
        });
    }
}
