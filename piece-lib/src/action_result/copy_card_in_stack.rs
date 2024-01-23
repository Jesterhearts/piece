use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    player::Controller,
    stack::{ActiveTarget, Stack},
};

#[derive(Debug, Clone)]
pub(crate) struct CopyCardInStack {
    pub(crate) card: CardId,
    pub(crate) controller: Controller,
    pub(crate) targets: Vec<Vec<ActiveTarget>>,
    pub(crate) x_is: Option<usize>,
    pub(crate) chosen_modes: Vec<usize>,
}

impl Action for CopyCardInStack {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            card,
            controller,
            targets,
            x_is,
            chosen_modes,
        } = self;
        let copy = card.token_copy_of(db, *controller);
        if let Some(x_is) = x_is {
            db[copy].x_is = *x_is;
        }
        Stack::push_card(db, *card, targets.clone(), chosen_modes.clone())
    }
}
