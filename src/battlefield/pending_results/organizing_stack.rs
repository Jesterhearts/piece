use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::{ActionResult, PendingResult, PendingResults},
    in_play::Database,
    player::AllPlayers,
    stack::StackEntry,
};

#[derive(Debug)]
pub struct OrganizingStack {
    pub entries: Vec<StackEntry>,
    pub choices: IndexSet<usize>,
}
impl OrganizingStack {
    pub(crate) fn new(entries: Vec<StackEntry>) -> Self {
        Self {
            entries,
            choices: Default::default(),
        }
    }
}

impl PendingResult for OrganizingStack {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        true
    }

    fn options(&self, db: &mut Database, _all_players: &AllPlayers) -> Vec<(usize, String)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(idx, _)| !self.choices.contains(idx))
            .map(|(idx, entry)| (idx, entry.display(db)))
            .collect_vec()
    }

    fn description(&self, _db: &Database) -> String {
        "stack order".to_string()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn make_choice(
        &mut self,
        _db: &mut Database,
        _all_players: &mut AllPlayers,
        choice: Option<usize>,
        results: &mut PendingResults,
    ) -> bool {
        if let Some(choice) = choice {
            self.choices.insert(choice);

            debug!("Chosen {:?}", self.choices);

            if self.choices.len() == self.entries.len() {
                let entries = self
                    .choices
                    .iter()
                    .map(|choice| self.entries[*choice].clone())
                    .collect_vec();

                results.push_settled(ActionResult::UpdateStackEntries(entries));
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}