use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct TapPermanent {
    pub(crate) card: CardId,
}

impl Action for TapPermanent {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card } = self;
        card.tap(db)
    }
}
