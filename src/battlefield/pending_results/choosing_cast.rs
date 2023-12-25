use itertools::Itertools;

use crate::{
    battlefield::{PendingResult, PendingResults},
    in_play::{CardId, Database},
    player::AllPlayers,
    stack::Stack,
};

#[derive(Debug)]
pub struct ChoosingCast {
    pub choosing_to_cast: Vec<CardId>,
    pub paying_costs: bool,
    pub discovering: bool,
}

impl PendingResult for ChoosingCast {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        true
    }

    fn options(&self, db: &mut Database, _all_players: &AllPlayers) -> Vec<(usize, String)> {
        self.choosing_to_cast
            .iter()
            .enumerate()
            .map(|(idx, card)| (idx, card.name(db)))
            .collect_vec()
    }

    fn description(&self, _db: &Database) -> String {
        "spells to cast".to_string()
    }

    fn is_empty(&self) -> bool {
        self.choosing_to_cast.is_empty()
    }

    fn make_choice(
        &mut self,
        db: &mut Database,
        _all_players: &mut AllPlayers,
        choice: Option<usize>,
        results: &mut PendingResults,
    ) -> bool {
        if let Some(choice) = choice {
            let cast_results = Stack::move_card_to_stack_from_exile(
                db,
                self.choosing_to_cast.remove(choice),
                self.paying_costs,
            );
            results.extend(cast_results);
            true
        } else {
            if self.discovering {
                let card = *self.choosing_to_cast.iter().exactly_one().unwrap();
                card.move_to_hand(db);
            }
            self.choosing_to_cast.clear();
            true
        }
    }
}
