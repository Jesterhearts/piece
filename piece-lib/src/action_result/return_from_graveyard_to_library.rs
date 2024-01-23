use crate::{
    action_result::Action, in_play::Database, library::Library, pending_results::PendingResults,
    stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ReturnFromGraveyardToLibrary {
    pub(crate) target: ActiveTarget,
}

impl Action for ReturnFromGraveyardToLibrary {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target } = self;
        let ActiveTarget::Graveyard { id: target } = target else {
            unreachable!()
        };

        Library::place_on_top(db, db[*target].owner, *target);
        PendingResults::default()
    }
}
