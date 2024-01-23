use crate::{
    action_result::Action,
    battlefield::{complete_add_from_library, move_card_to_battlefield},
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct AddToBattlefieldSkipReplacementEffectsFromLibrary {
    pub(crate) card: CardId,
    pub(crate) enters_tapped: bool,
}

impl Action for AddToBattlefieldSkipReplacementEffectsFromLibrary {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            card,
            enters_tapped,
        } = self;
        let mut results = PendingResults::default();
        move_card_to_battlefield(db, *card, *enters_tapped, &mut results, None);
        complete_add_from_library(db, *card, &mut results);
        results
    }
}
