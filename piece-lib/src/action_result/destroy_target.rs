use crate::{
    action_result::Action, battlefield::Battlefields, in_play::Database,
    pending_results::PendingResults, stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct DestroyTarget {
    pub(crate) target: ActiveTarget,
}

impl Action for DestroyTarget {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target } = self;
        let ActiveTarget::Battlefield { id } = target else {
            unreachable!()
        };

        Battlefields::permanent_to_graveyard(db, *id)
    }
}
