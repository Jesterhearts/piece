use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct Transform {
    pub(crate) target: CardId,
}

impl Action for Transform {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target } = self;
        target.transform(db);

        PendingResults::default()
    }
}
