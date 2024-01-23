use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    library::Library,
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct MoveFromLibraryToBottomOfLibrary {
    pub(crate) card: CardId,
}

impl Action for MoveFromLibraryToBottomOfLibrary {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card } = self;

        let owner = db[*card].owner;
        db.all_players[owner].library.remove(*card);
        Library::place_on_bottom(db, owner, *card);
        PendingResults::default()
    }
}
