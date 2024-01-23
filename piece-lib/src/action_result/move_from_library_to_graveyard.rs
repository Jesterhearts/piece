use crate::{action_result::Action, battlefield::Battlefields, in_play::CardId};

#[derive(Debug, Clone)]
pub(crate) struct MoveFromLibraryToGraveyard {
    pub(crate) card: CardId,
}

impl Action for MoveFromLibraryToGraveyard {
    fn apply(&self, db: &mut crate::in_play::Database) -> crate::pending_results::PendingResults {
        let Self { card } = self;
        Battlefields::library_to_graveyard(db, *card)
    }
}
