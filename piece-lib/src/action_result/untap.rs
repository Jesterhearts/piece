use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    protogen::counters::Counter,
};

#[derive(Debug, Clone)]
pub(crate) struct Untap {
    pub(crate) target: CardId,
}

impl Action for Untap {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target } = self;
        let stun = db[*target].counters.entry(Counter::STUN).or_default();
        if *stun > 0 {
            *stun -= 1;
        } else {
            target.untap(db);
        }

        PendingResults::default()
    }
}
