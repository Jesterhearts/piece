use itertools::Itertools;

use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    protogen::effects::Duration,
    stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ExileGraveyard {
    pub(crate) target: ActiveTarget,
    pub(crate) source: CardId,
}

impl Action for ExileGraveyard {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target, source } = self;
        let ActiveTarget::Player { id } = target else {
            unreachable!()
        };

        for card in db.graveyard[*id].iter().copied().collect_vec() {
            card.move_to_exile(db, *source, None, Duration::PERMANENTLY)
        }

        PendingResults::default()
    }
}
