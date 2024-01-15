use itertools::Itertools;

use crate::{
    in_play::{CardId, Database},
    pending_results::{PendingResult, PendingResults},
    stack::Stack,
};

#[derive(Debug)]
pub(crate) struct ChoosingCast {
    pub(crate) choosing_to_cast: Vec<CardId>,
    pub(crate) paying_costs: bool,
    pub(crate) discovering: bool,
}

impl PendingResult for ChoosingCast {
    fn optional(&self, _db: &Database) -> bool {
        true
    }

    fn options(&self, db: &mut Database) -> Vec<(usize, String)> {
        self.choosing_to_cast
            .iter()
            .enumerate()
            .map(|(idx, card)| (idx, card.name(db).clone()))
            .collect_vec()
    }

    fn description(&self, _db: &Database) -> String {
        "spells to cast".to_string()
    }

    fn target_for_option(
        &self,
        db: &Database,
        option: usize,
    ) -> Option<crate::stack::ActiveTarget> {
        self.choosing_to_cast
            .get(option)
            .and_then(|card| card.target_from_location(db))
    }

    fn is_empty(&self) -> bool {
        self.choosing_to_cast.is_empty()
    }

    fn make_choice(
        &mut self,
        db: &mut Database,
        choice: Option<usize>,
        results: &mut PendingResults,
    ) -> bool {
        if let Some(choice) = choice {
            let cast_results = Stack::move_card_to_stack_from_exile(
                db,
                self.choosing_to_cast.remove(choice),
                self.paying_costs,
            );
            if cast_results.is_empty() && self.discovering {
                let card = *self.choosing_to_cast.iter().exactly_one().unwrap();
                card.move_to_hand(db);
            } else {
                results.extend(cast_results);
            }
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
