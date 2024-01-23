use rand::{seq::SliceRandom, thread_rng};

use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    library::Library,
    pending_results::PendingResults,
    player::Controller,
};

#[derive(Debug, Clone)]
pub(crate) struct CascadeExileToBottomOfLibrary {
    pub(crate) player: Controller,
}

impl Action for CascadeExileToBottomOfLibrary {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { player } = self;
        let mut cards = CardId::exiled_with_cascade(db);
        cards.shuffle(&mut thread_rng());

        for card in cards {
            Library::place_on_bottom(db, (*player).into(), card);
        }
        PendingResults::default()
    }
}
