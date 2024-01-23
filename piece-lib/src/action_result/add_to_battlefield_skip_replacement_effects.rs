use crate::{
    action_result::Action,
    battlefield::{complete_add_from_stack_or_hand, move_card_to_battlefield},
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct AddToBattlefieldSkipReplacementEffects {
    pub(crate) card: CardId,
    pub(crate) aura_target: Option<CardId>,
}

impl Action for AddToBattlefieldSkipReplacementEffects {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card, aura_target } = self;
        let mut results = PendingResults::default();
        move_card_to_battlefield(db, *card, false, &mut results, *aura_target);
        complete_add_from_stack_or_hand(db, *card, &mut results);
        results
    }
}
