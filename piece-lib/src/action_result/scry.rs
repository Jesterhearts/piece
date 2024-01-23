use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
};

#[derive(Debug, Clone)]
pub(crate) struct Scry {
    pub(crate) source: CardId,
    pub(crate) count: u32,
}

impl Action for Scry {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { source, count } = self;
        let mut cards = vec![];
        for _ in 0..*count {
            let controller = db[*source].controller;
            if let Some(card) = db.all_players[controller].library.draw() {
                cards.push(card);
            } else {
                break;
            }
        }

        let mut results = PendingResults::default();
        results.push_choose_scry(cards);

        results
    }
}
