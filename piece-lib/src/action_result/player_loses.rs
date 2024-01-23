use crate::{
    action_result::Action, in_play::Database, pending_results::PendingResults, player::Owner,
};

#[derive(Debug, Clone)]
pub(crate) struct PlayerLoses {
    pub(crate) player: Owner,
}

impl Action for PlayerLoses {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { player } = self;
        db.all_players[*player].lost = true;
        PendingResults::default()
    }
}
