use crate::{
    action_result::Action,
    in_play::Database,
    pending_results::PendingResults,
    stack::{StackEntry, StackId},
};

#[derive(Debug, Clone)]
pub(crate) struct UpdateStackEntries {
    pub(crate) entries: Vec<StackEntry>,
}

impl Action for UpdateStackEntries {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { entries } = self;
        db.stack.entries = entries
            .iter()
            .map(|e| (StackId::new(), e.clone()))
            .collect();
        db.stack.settle();
        PendingResults::default()
    }
}
