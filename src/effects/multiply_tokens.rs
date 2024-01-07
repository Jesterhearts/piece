use crate::{
    battlefield::create_token_copy_with_replacements,
    effects::{EffectBehaviors, ReplacementEffect},
    protogen,
};

#[derive(Debug, Clone)]
pub(crate) struct MultiplyTokens {
    multiplier: usize,
}

impl TryFrom<&protogen::effects::MultiplyTokens> for MultiplyTokens {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::MultiplyTokens) -> Result<Self, Self::Error> {
        Ok(Self {
            multiplier: usize::try_from(value.multiplier)?,
        })
    }
}

impl EffectBehaviors for MultiplyTokens {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        _results: &mut crate::pending_results::PendingResults,
    ) {
        unreachable!()
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        _results: &mut crate::pending_results::PendingResults,
    ) {
        unreachable!()
    }

    fn replace_token_creation(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        replacements: &mut std::vec::IntoIter<(crate::in_play::CardId, ReplacementEffect)>,
        token: crate::in_play::CardId,
        modifiers: &[super::ModifyBattlefield],
        results: &mut crate::pending_results::PendingResults,
    ) {
        for _ in 0..self.multiplier {
            create_token_copy_with_replacements(
                db,
                source,
                token,
                modifiers,
                &mut replacements.clone(),
                results,
            )
        }
    }
}
