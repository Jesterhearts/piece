use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct MoveToHandFromLibrary {
    pub(crate) card: CardId,
}

impl Action for MoveToHandFromLibrary {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card } = self;
        card.move_to_hand(db);
        PendingResults::default()
    }
}
