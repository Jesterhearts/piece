use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    protogen::effects::SelectMode,
    stack::Selected,
};

impl EffectBehaviors for SelectMode {
    fn wants_input(
        &self,
        _db: &Database,
        _source: Option<CardId>,
        _already_selected: &[Selected],
        _modes: &[usize],
    ) -> bool {
        true
    }

    fn options(
        &self,
        _db: &Database,
        _source: Option<CardId>,
        _already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        Options::MandatoryList(self.descriptions.iter().cloned().enumerate().collect_vec())
    }

    fn select(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if let Some(option) = option {
            selected.modes.push(option);
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        // Already selected modes
        vec![]
    }
}
