use crate::{
    action_result::Action, in_play::Database, pending_results::PendingResults, player::Controller,
};

#[derive(Debug, Clone)]
pub(crate) struct LoseLife {
    pub(crate) target: Controller,
    pub(crate) count: u32,
}

impl Action for LoseLife {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target, count } = self;
        db.all_players[*target].life_total -= *count as i32;
        PendingResults::default()
    }
}
