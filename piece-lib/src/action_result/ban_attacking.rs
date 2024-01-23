use crate::{
    action_result::Action, in_play::Database, pending_results::PendingResults, player::Owner,
};

#[derive(Debug, Clone)]
pub(crate) struct BanAttacking {
    pub(crate) banned: Owner,
}

impl Action for BanAttacking {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { banned } = self;
        db.all_players[*banned].ban_attacking_this_turn = true;
        PendingResults::default()
    }
}
