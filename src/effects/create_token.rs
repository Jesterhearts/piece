use crate::{
    battlefield::ActionResult,
    effects::{EffectBehaviors, Token},
    protogen,
};

#[derive(Debug, Clone)]
pub struct CreateToken {
    token: Token,
}

impl TryFrom<&protogen::effects::CreateToken> for CreateToken {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::CreateToken) -> Result<Self, Self::Error> {
        Ok(Self {
            token: value.try_into()?,
        })
    }
}

impl EffectBehaviors for CreateToken {
    fn needs_targets(&self) -> usize {
        0
    }

    fn wants_targets(&self) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::CreateToken {
            source: controller,
            token: Box::new(self.token.clone()),
        });
    }

    fn push_behavior_from_top_of_library(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        _target: crate::in_play::CardId,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::CreateToken {
            source: source.controller(db),
            token: Box::new(self.token.clone()),
        });
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::CreateToken {
            source: source.controller(db),
            token: Box::new(self.token.clone()),
        });
    }
}