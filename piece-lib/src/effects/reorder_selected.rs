use itertools::Itertools;

use crate::{
    effects::{
        ApplyResult, EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult,
    },
    in_play::{CardId, Database},
    protogen::effects::ReorderSelected,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for ReorderSelected {
    fn wants_input(
        &self,
        _db: &Database,
        _source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> bool {
        already_selected.len() > 1
    }

    fn options(
        &self,
        db: &Database,
        _source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        if already_selected.len() <= 1 {
            return Options::OptionalList(vec![]);
        }

        let start_at = self.reordering as usize;
        let (_, options) = already_selected.split_at(start_at);

        let mut results = vec![];
        for (idx, option) in options.iter().enumerate() {
            let idx = idx + start_at;
            results.push((idx, option.display(db)))
        }

        Options::ListWithDefault(results)
    }

    fn select(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if let Some(option) = option {
            selected.swap(self.reordering as usize, option);
            self.reordering += 1;
            if self.reordering as usize == selected.len() {
                SelectionResult::Complete
            } else {
                SelectionResult::PendingChoice
            }
        } else {
            SelectionResult::Complete
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut replaced = vec![(*self.associated_effect).clone()];
        let mut target_stack_index = 0;
        for selected in selected.drain(..) {
            match &selected.target_type {
                TargetType::Stack(entry) => {
                    let swapping = db.stack.entries.get_index_of(entry).unwrap();
                    db.stack.entries.swap_indices(target_stack_index, swapping);
                    target_stack_index += 1;
                }
                TargetType::ReplacementAbility(replacement) => {
                    for replacement in replacement.effects.iter() {
                        replaced = replaced
                            .into_iter()
                            .flat_map(|effect| {
                                replacement
                                    .effect
                                    .as_ref()
                                    .unwrap()
                                    .apply_replacement(effect)
                            })
                            .collect_vec()
                    }
                }
                _ => unreachable!(),
            }
        }

        let _ = selected.restore();
        vec![ApplyResult::PushFront(EffectBundle {
            source,
            effects: replaced,
            skip_replacement: true,
            ..Default::default()
        })]
    }
}
