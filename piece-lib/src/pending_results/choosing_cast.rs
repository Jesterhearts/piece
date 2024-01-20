use itertools::Itertools;

use crate::{
    in_play::{CastFrom, Database},
    pending_results::{Options, PendingResult, PendingResults},
    protogen::{ids::CardId, targets::Location},
    stack::add_card_to_stack,
};

#[derive(Debug)]
pub(crate) struct ChoosingCast {
    pub(crate) choosing_to_cast: Vec<CardId>,
    pub(crate) paying_costs: bool,
    pub(crate) discovering: bool,
}

impl PendingResult for ChoosingCast {
    fn cancelable(&self, _db: &Database) -> bool {
        true
    }

    fn options(&self, db: &mut Database) -> Options {
        Options::OptionalList(
            self.choosing_to_cast
                .iter()
                .enumerate()
                .map(|(idx, card)| (idx, card.name(db).clone()))
                .collect_vec(),
        )
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
            let card = self.choosing_to_cast.remove(choice);
            let cast_from = match card.location(db).unwrap() {
                Location::IN_HAND => CastFrom::Hand,
                Location::IN_GRAVEYARD => CastFrom::Graveyard,
                Location::IN_EXILE => CastFrom::Exile,
                _ => unreachable!(),
            };
            let cast_results =
                add_card_to_stack(db, card.clone(), Some(cast_from), self.paying_costs);
            if cast_results.is_empty() && self.discovering {
                card.move_to_hand(db);
            } else {
                results.extend(cast_results);
            }
            true
        } else {
            if self.discovering {
                let card = self.choosing_to_cast.iter().exactly_one().cloned().unwrap();
                card.move_to_hand(db);
            }
            self.choosing_to_cast.clear();
            true
        }
    }
}
