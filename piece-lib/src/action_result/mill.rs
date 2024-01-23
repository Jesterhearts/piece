use crate::{
    action_result::Action, battlefield::Battlefields, in_play::Database,
    pending_results::PendingResults, stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct Mill {
    pub(crate) count: u32,
    pub(crate) target: ActiveTarget,
}

impl Action for Mill {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { count, target } = self;
        let mut pending = PendingResults::default();
        let ActiveTarget::Player { id: target } = target else {
            unreachable!()
        };

        for _ in 0..*count {
            let card_id = db.all_players[*target].library.draw();
            if let Some(card_id) = card_id {
                pending.extend(Battlefields::library_to_graveyard(db, card_id));
            }
        }

        pending
    }
}
