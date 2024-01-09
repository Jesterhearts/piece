use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors},
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct Mill {
    count: usize,
    restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::Mill> for Mill {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Mill) -> Result<Self, Self::Error> {
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

impl EffectBehaviors for Mill {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        controller: crate::player::Controller,
        _already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        db.all_players
            .all_players()
            .into_iter()
            .filter(|player| {
                player.passes_restrictions(db, log_session, controller, &self.restrictions)
            })
            .map(|player| ActiveTarget::Player { id: player })
            .collect_vec()
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let valid_targets = self.valid_targets(
            db,
            source,
            crate::log::LogId::current(db),
            controller,
            results.all_currently_targeted(),
        );

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            crate::log::LogId::current(db),
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
        results.push_settled(ActionResult::Mill {
            count: self.count,
            targets,
        });
    }
}
