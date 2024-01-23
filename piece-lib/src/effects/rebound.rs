use crate::{
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    player::Controller,
    protogen::effects::Rebound,
    stack::ActiveTarget,
};

impl EffectBehaviors for Rebound {
    fn needs_targets(&self, _db: &Database, _source: CardId) -> usize {
        0
    }

    fn wants_targets(&self, _db: &Database, _source: CardId) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    ) {
        todo!()
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    ) {
        todo!()
    }
}
