use crate::{
    action_result::Action,
    in_play::Database,
    pending_results::PendingResults,
    player::{Controller, Player},
};

#[derive(Debug, Clone)]
pub(crate) struct DrawCards {
    pub(crate) target: Controller,
    pub(crate) count: usize,
}

impl Action for DrawCards {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target, count } = self;
        Player::draw(db, (*target).into(), *count)
    }
}
