use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    protogen::counters::Counter,
};

#[derive(Debug, Clone)]
pub(crate) struct RemoveCounters {
    pub(crate) target: CardId,
    pub(crate) counter: protobuf::EnumOrUnknown<Counter>,
    pub(crate) count: usize,
}

impl Action for RemoveCounters {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            target,
            counter,
            count,
        } = self;
        *db[*target]
            .counters
            .entry(counter.enum_value().unwrap())
            .or_default() = db[*target]
            .counters
            .entry(counter.enum_value().unwrap())
            .or_default()
            .saturating_sub(*count);
        PendingResults::default()
    }
}
