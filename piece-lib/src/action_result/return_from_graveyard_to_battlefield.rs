use crate::{
    action_result::Action, battlefield::Battlefields, in_play::Database,
    pending_results::PendingResults, stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ReturnFromGraveyardToBattlefield {
    pub(crate) target: ActiveTarget,
}

impl Action for ReturnFromGraveyardToBattlefield {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target } = self;
        let mut pending = PendingResults::default();
        let ActiveTarget::Graveyard { id: target } = target else {
            unreachable!()
        };
        pending.extend(Battlefields::add_from_stack_or_hand(db, *target, None));

        pending
    }
}
