use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::Database,
    pending_results::PendingResults,
    protogen::ids::Controller,
    protogen::{effects::GainCounters, ids::CardId},
    stack::ActiveTarget,
};

impl EffectBehaviors for GainCounters {
    fn needs_targets(&self, _db: &Database, _source: &CardId) -> usize {
        0
    }

    fn wants_targets(&self, _db: &Database, _source: &CardId) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut Database,
        source: &CardId,
        _controller: &Controller,
        results: &mut PendingResults,
    ) {
        results.push_settled(ActionResult::AddCounters {
            source: source.clone(),
            target: source.clone(),
            count: self.count.clone(),
            counter: self.counter,
        });
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut Database,
        _targets: Vec<ActiveTarget>,
        source: &CardId,
        _controller: &Controller,
        results: &mut PendingResults,
    ) {
        results.push_settled(ActionResult::AddCounters {
            source: source.clone(),
            target: source.clone(),
            count: self.count.clone(),
            counter: self.counter,
        })
    }
}
