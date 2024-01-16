use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    player::Controller,
    protogen::effects::GainCounters,
    stack::ActiveTarget,
};

impl EffectBehaviors for GainCounters {
    fn needs_targets(&self, _db: &Database, _source: CardId) -> usize {
        0
    }

    fn wants_targets(&self, _db: &Database, _source: CardId) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut Database,
        source: CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        results.push_settled(ActionResult::AddCounters {
            source,
            target: source,
            count: self.count.count.as_ref().unwrap().clone(),
            counter: self.counter,
        });
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut Database,
        _targets: Vec<ActiveTarget>,
        _apply_to_self: bool,
        source: CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        results.push_settled(ActionResult::AddCounters {
            source,
            target: source,
            count: self.count.count.as_ref().unwrap().clone(),
            counter: self.counter,
        })
    }
}
