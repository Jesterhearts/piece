use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    protogen::targets::Location,
};

#[derive(Debug, Clone)]
pub(crate) struct Discard {
    pub(crate) card: CardId,
}

impl Action for Discard {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card } = self;
        assert!(card.is_in_location(db, Location::IN_HAND));
        card.move_to_graveyard(db);
        PendingResults::default()
    }
}
