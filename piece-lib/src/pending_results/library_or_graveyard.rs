use itertools::Itertools;

use crate::{
    in_play::{CardId, Database},
    library::Library,
    pending_results::{Options, PendingResult, PendingResults},
};

#[derive(Debug)]
pub(crate) struct LibraryOrGraveyard {
    pub(crate) card: CardId,
}

impl PendingResult for LibraryOrGraveyard {
    fn cancelable(&self, _db: &Database) -> bool {
        false
    }

    fn options(&self, _db: &mut Database) -> Options {
        Options::MandatoryList(
            ["Library".to_string(), "Graveyard".to_string()]
                .into_iter()
                .enumerate()
                .collect_vec(),
        )
    }

    fn target_for_option(
        &self,
        _db: &Database,
        _option: usize,
    ) -> Option<crate::stack::ActiveTarget> {
        None
    }

    fn description(&self, db: &crate::in_play::Database) -> String {
        self.card.name(db).clone()
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn make_choice(
        &mut self,
        db: &mut Database,
        choice: Option<usize>,
        _results: &mut PendingResults,
    ) -> bool {
        if let Some(choice) = choice {
            match choice {
                0 => Library::place_on_top(db, db[self.card].controller.into(), self.card),
                1 => self.card.move_to_graveyard(db),
                _ => unreachable!(),
            }
            true
        } else {
            false
        }
    }
}
