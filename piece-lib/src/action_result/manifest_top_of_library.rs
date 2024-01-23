use crate::{
    action_result::Action,
    in_play::Database,
    pending_results::PendingResults,
    player::{Controller, Player},
};

#[derive(Debug, Clone)]
pub(crate) struct ManifestTopOfLibrary {
    pub(crate) player: Controller,
}

impl Action for ManifestTopOfLibrary {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { player } = self;
        Player::manifest(db, (*player).into())
    }
}
