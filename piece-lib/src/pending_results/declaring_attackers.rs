use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    action_result::{declare_attackers::DeclareAttackers, ActionResult},
    in_play::{CardId, Database},
    pending_results::{Options, PendingResult, PendingResults},
    player::Owner,
};

#[derive(Debug)]
pub(crate) struct DeclaringAttackers {
    pub(super) candidates: Vec<CardId>,
    pub(super) choices: IndexSet<usize>,
    pub(super) targets: Vec<Owner>,
    pub(super) valid_targets: Vec<Owner>,
}

impl PendingResult for DeclaringAttackers {
    fn cancelable(&self, _db: &Database) -> bool {
        true
    }

    fn options(&self, db: &mut Database) -> Options {
        if self.choices.len() == self.targets.len() {
            Options::OptionalList(
                self.candidates
                    .iter()
                    .map(|card| card.name(db).clone())
                    .enumerate()
                    .filter(|(idx, _)| !self.choices.contains(idx))
                    .collect_vec(),
            )
        } else {
            Options::MandatoryList(
                self.valid_targets
                    .iter()
                    .map(|player| db.all_players[*player].name.clone())
                    .enumerate()
                    .collect_vec(),
            )
        }
    }

    fn target_for_option(
        &self,
        _db: &Database,
        option: usize,
    ) -> Option<crate::stack::ActiveTarget> {
        if self.choices.len() == self.targets.len() {
            self.candidates
                .get(option)
                .map(|card| crate::stack::ActiveTarget::Battlefield { id: *card })
        } else {
            self.valid_targets
                .get(option)
                .map(|t| crate::stack::ActiveTarget::Player { id: *t })
        }
    }

    fn description(&self, _db: &crate::in_play::Database) -> String {
        "attackers".to_string()
    }

    fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    fn make_choice(
        &mut self,
        _db: &mut Database,
        choice: Option<usize>,
        results: &mut PendingResults,
    ) -> bool {
        if let Some(choice) = choice {
            if self.candidates.is_empty() {
                true
            } else {
                if self.choices.len() == self.targets.len() {
                    if !self.choices.insert(choice) {
                        return true;
                    }
                } else {
                    self.targets.push(self.valid_targets[choice]);
                }
                false
            }
        } else if self.choices.len() == self.targets.len() {
            results.push_settled(ActionResult::from(DeclareAttackers {
                attackers: self
                    .choices
                    .iter()
                    .map(|choice| self.candidates[*choice])
                    .collect_vec(),
                targets: self.targets.clone(),
            }));
            true
        } else {
            false
        }
    }
}
