use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    in_play::Database,
    pending_results::{PendingResult, PendingResults},
    stack::StackEntry,
};

#[derive(Debug)]
pub(crate) struct OrganizingStack {
    pub(crate) entries: Vec<StackEntry>,
    pub(crate) choices: IndexSet<usize>,
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
    fn optional(&self, _db: &Database) -> bool {
        true
    }

    fn options(&self, db: &mut Database) -> Vec<(usize, String)> {
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
