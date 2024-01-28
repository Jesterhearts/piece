use itertools::Itertools;

use crate::{
    effects::{ApplyResult, EffectBehaviors, Options, SelectedStack, SelectionResult},
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
        Options::MandatoryList(
            self.modes
                .iter()
                .map(|mode| mode.oracle_text.clone())
                .enumerate()
                .collect_vec(),
        )
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
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut pending = vec![];
        for mode in selected.modes.clone() {
            pending.extend(self.modes[mode].effect.as_mut().unwrap().apply(
                db,
                source,
                selected,
                skip_replacement,
            ));
        }

        pending
    }
}
