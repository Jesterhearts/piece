use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, Options, PendingEffects, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    protogen::effects::ReorderSelected,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for ReorderSelected {
    fn options(
        &self,
        db: &Database,
        _source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
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
        _modes: &mut Vec<usize>,
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
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        modes: &[usize],
        _skip_replacement: bool,
    ) {
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

        for mut effect in replaced {
            effect
                .effect
                .as_mut()
                .unwrap()
                .apply(db, pending, source, selected, modes, true);
        }
    }
}
