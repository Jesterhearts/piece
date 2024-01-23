use crate::{
    action_result::Action,
    in_play::Database,
    pending_results::{examine_top_cards::ExamineCards, PendingResults},
    player::Controller,
    protogen::{effects::examine_top_cards::Dest, targets::Location},
};

#[derive(Debug, Clone)]
pub(crate) struct ExamineTopCards {
    pub(crate) destinations: Vec<Dest>,
    pub(crate) count: u32,
    pub(crate) controller: Controller,
}

impl Action for ExamineTopCards {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            destinations,
            count,
            controller,
        } = self;
        let mut cards = vec![];
        for _ in 0..*count {
            if let Some(card) = db.all_players[*controller].library.draw() {
                cards.push(card);
            } else {
                break;
            }
        }

        let mut results = PendingResults::default();
        results.push_examine_cards(ExamineCards::new(
            Location::IN_LIBRARY,
            cards,
            destinations.clone(),
        ));

        results
    }
}
