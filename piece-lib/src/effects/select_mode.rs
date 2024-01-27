use itertools::Itertools;

use crate::{
    effects::{
        ApplyResult, EffectBehaviors, EffectBundle, Options, SelectedStack,
        SelectionResult,
    },
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
                .map(|mode| {
                    mode.effects
                        .iter()
                        .map(|effect| &effect.oracle_text)
                        .join(" ")
                })
                .enumerate()
                .collect_vec(),
        )
    }

    fn select(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        option: Option<usize>,
        _selected: &mut SelectedStack,
        modes: &mut Vec<usize>,
    ) -> SelectionResult {
        if let Some(option) = option {
            modes.push(option);
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        _db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        modes: &[usize],
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut pending = vec![];
        for mode in modes {
            pending.push(ApplyResult::PushBack(EffectBundle {
                selected: selected.clone(),
                source,
                effects: self.modes[*mode].effects.clone(),
                ..Default::default()
            }));
        }

        pending
    }
}
