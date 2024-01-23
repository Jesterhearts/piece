use crate::{
    action_result::Action,
    battlefield::Battlefields,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct AddToBattlefield {
    pub(crate) card: CardId,
    pub(crate) aura_target: Option<CardId>,
}

impl Action for AddToBattlefield {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card, aura_target } = self;

        Battlefields::add_from_stack_or_hand(db, *card, *aura_target)
    }
}
