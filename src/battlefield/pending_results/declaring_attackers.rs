use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::{ActionResult, PendingResult, PendingResults},
    in_play::{CardId, Database},
    player::{AllPlayers, Owner},
};

#[derive(Debug)]
pub(crate) struct DeclaringAttackers {
    pub(super) candidates: Vec<CardId>,
    pub(super) choices: IndexSet<usize>,
    pub(super) targets: Vec<Owner>,
    pub(super) valid_targets: Vec<Owner>,
}

impl PendingResult for DeclaringAttackers {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        true
    }

    fn options(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        if self.choices.len() == self.targets.len() {
            self.candidates
                .iter()
                .map(|card| card.name(db))
                .enumerate()
                .filter(|(idx, _)| !self.choices.contains(idx))
                .collect_vec()
        } else {
            self.valid_targets
                .iter()
                .map(|player| all_players[*player].name.clone())
                .enumerate()
                .collect_vec()
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
        _all_players: &mut AllPlayers,
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
            results.push_settled(ActionResult::DeclareAttackers {
                attackers: self
                    .choices
                    .iter()
                    .map(|choice| self.candidates[*choice])
                    .collect_vec(),
                targets: self.targets.clone(),
            });
            true
        } else {
            false
        }
    }
}
