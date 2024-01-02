use itertools::Itertools;

use crate::{
    battlefield::ActionResult, effects::EffectBehaviors, player::AllPlayers, protogen,
    stack::ActiveTarget, targets::Restriction,
};

#[derive(Debug)]
pub(crate) struct CantAttackThisTurn {
    retrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::CantAttackThisTurn> for CantAttackThisTurn {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::CantAttackThisTurn) -> Result<Self, Self::Error> {
        Ok(Self {
            retrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for CantAttackThisTurn {
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

    fn valid_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        AllPlayers::all_players_in_db(db)
            .into_iter()
            .filter(|player| {
                player.passes_restrictions(db, controller, &source.restrictions(db))
                    && player.passes_restrictions(db, controller, &self.retrictions)
            })
            .map(|player| ActiveTarget::Player { id: player })
            .collect_vec()
    }

    fn push_pending_behavior(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        _results: &mut crate::battlefield::PendingResults,
    ) {
        unreachable!()
    }

    fn push_behavior_with_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        for target in targets {
            let ActiveTarget::Player { id } = target else {
                warn!("Skipping target {:?}", target);
                continue;
            };

            results.push_settled(ActionResult::BanAttacking(id));
        }
    }
}
