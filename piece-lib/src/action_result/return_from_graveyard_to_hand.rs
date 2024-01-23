use crate::{
    action_result::Action, in_play::Database, pending_results::PendingResults, stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ReturnFromGraveyardToHand {
    pub(crate) target: ActiveTarget,
}

impl Action for ReturnFromGraveyardToHand {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target } = self;
        let ActiveTarget::Hand { id } = target else {
            unreachable!()
        };

        id.move_to_hand(db);

        PendingResults::default()
    }
}
