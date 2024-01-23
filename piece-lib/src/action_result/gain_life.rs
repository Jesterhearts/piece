use crate::{
    action_result::Action, in_play::Database, pending_results::PendingResults, player::Controller,
};

#[derive(Debug, Clone)]
pub(crate) struct GainLife {
    pub(crate) target: Controller,
    pub(crate) count: u32,
}

impl Action for GainLife {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target, count } = self;

        db.all_players[*target].life_total += *count as i32;
        db.all_players[*target].life_gained_this_turn += *count;
        PendingResults::default()
    }
}
