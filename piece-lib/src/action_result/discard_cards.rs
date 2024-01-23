use itertools::Itertools;

use crate::{
    action_result::Action, in_play::Database, pending_results::PendingResults, player::Controller,
};

#[derive(Debug, Clone)]
pub(crate) struct DiscardCards {
    pub(crate) target: Controller,
    pub(crate) count: u32,
}

impl Action for DiscardCards {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target, count } = self;
        let mut pending = PendingResults::default();
        pending.push_choose_discard(db.hand[*target].iter().copied().collect_vec(), *count);
        pending
    }
}
