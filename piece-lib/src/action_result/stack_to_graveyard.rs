use crate::{
    action_result::Action,
    battlefield::Battlefields,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct StackToGraveyard {
    pub(crate) card: CardId,
}

impl Action for StackToGraveyard {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card } = self;
        Battlefields::stack_to_graveyard(db, *card)
    }
}
