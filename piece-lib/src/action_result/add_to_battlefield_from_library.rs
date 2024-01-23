use crate::{
    action_result::Action,
    battlefield::Battlefields,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct AddToBattlefieldFromLibrary {
    pub(crate) card: CardId,
    pub(crate) enters_tapped: bool,
}

impl Action for AddToBattlefieldFromLibrary {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            card,
            enters_tapped,
        } = self;
        Battlefields::add_from_library(db, *card, *enters_tapped)
    }
}
