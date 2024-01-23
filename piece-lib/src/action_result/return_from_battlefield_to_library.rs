use crate::{
    action_result::Action, in_play::Database, library::Library, pending_results::PendingResults,
    stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ReturnFromBattlefieldToLibrary {
    pub(crate) target: ActiveTarget,
    pub(crate) under_cards: u32,
}

impl Action for ReturnFromBattlefieldToLibrary {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            target,
            under_cards,
        } = self;
        let ActiveTarget::Battlefield { id: target } = target else {
            unreachable!()
        };

        Library::place_under_top(db, db[*target].owner, *target, *under_cards as usize);
        PendingResults::default()
    }
}
