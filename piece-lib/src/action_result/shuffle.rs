use crate::{
    action_result::Action, in_play::Database, pending_results::PendingResults, player::Owner,
};

#[derive(Debug, Clone)]
pub(crate) struct Shuffle {
    pub(crate) player: Owner,
}

impl Action for Shuffle {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { player } = self;
        db.all_players[*player].library.shuffle();
        PendingResults::default()
    }
}
