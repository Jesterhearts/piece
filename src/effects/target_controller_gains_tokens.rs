use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    effects::{EffectBehaviors, Token},
    protogen,
};

#[derive(Debug, Clone)]
pub(crate) struct TargetControllerGainsTokens {
    token: Token,
}

impl TryFrom<&protogen::effects::CreateToken> for TargetControllerGainsTokens {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::CreateToken) -> Result<Self, Self::Error> {
        Ok(Self {
            token: value.try_into()?,
        })
    }
}

impl EffectBehaviors for TargetControllerGainsTokens {
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
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        _results: &mut crate::battlefield::PendingResults,
    ) {
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
        results.push_settled(ActionResult::CreateToken {
            source: targets.into_iter().exactly_one().unwrap().id().unwrap(),
            token: self.token.clone(),
        });
    }
}
