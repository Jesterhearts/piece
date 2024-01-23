use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct CloneCard {
    pub(crate) cloning: CardId,
    pub(crate) cloned: CardId,
}

impl Action for CloneCard {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { cloning, cloned } = self;

        cloning.clone_card(db, *cloned);
        PendingResults::default()
    }
}
