use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct RevealCard {
    pub(crate) card: CardId,
}

impl Action for RevealCard {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card } = self;
        db[*card].revealed = true;
        PendingResults::default()
    }
}
