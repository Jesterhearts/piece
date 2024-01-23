use crate::{
    action_result::Action,
    in_play::{Database, ModifierId},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct ApplyToBattlefield {
    pub(crate) modifier: ModifierId,
}

impl Action for ApplyToBattlefield {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { modifier } = self;
        modifier.activate(&mut db.modifiers);
        PendingResults::default()
    }
}
