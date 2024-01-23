use crate::{
    action_result::Action,
    battlefield::Battlefields,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct PermanentToGraveyard {
    pub(crate) card: CardId,
}

impl Action for PermanentToGraveyard {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card } = self;
        Battlefields::permanent_to_graveyard(db, *card)
    }
}
