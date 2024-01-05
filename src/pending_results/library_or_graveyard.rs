use itertools::Itertools;

use crate::{
    in_play::{CardId, Database},
    pending_results::{PendingResult, PendingResults},
    player::AllPlayers,
};

#[derive(Debug)]
pub(crate) struct LibraryOrGraveyard {
    pub(crate) card: CardId,
}

impl PendingResult for LibraryOrGraveyard {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        false
    }

    fn options(&self, _db: &mut Database, _all_players: &AllPlayers) -> Vec<(usize, String)> {
        ["Library".to_string(), "Graveyard".to_string()]
            .into_iter()
            .enumerate()
            .collect_vec()
    }

    fn description(&self, db: &crate::in_play::Database) -> String {
        self.card.name(db)
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn make_choice(
        &mut self,
        db: &mut Database,
        all_players: &mut AllPlayers,
        choice: Option<usize>,
        _results: &mut PendingResults,
    ) -> bool {
        if let Some(choice) = choice {
            match choice {
                0 => all_players[self.card.owner(db)]
                    .deck
                    .place_on_top(db, self.card),
                1 => self.card.move_to_graveyard(db),
                _ => unreachable!(),
            }
            true
        } else {
            false
        }
    }
}
