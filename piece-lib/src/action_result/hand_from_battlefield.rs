use crate::{
    action_result::Action,
    battlefield::Battlefields,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct HandFromBattlefield {
    pub(crate) card: CardId,
}

impl Action for HandFromBattlefield {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { card } = self;
        Battlefields::permanent_to_hand(db, *card)
    }
}
