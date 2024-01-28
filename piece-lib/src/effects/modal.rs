use crate::{
    effects::{ApplyResult, EffectBehaviors, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    protogen::effects::Modal,
    stack::Selected,
};

impl EffectBehaviors for Modal {
    fn priority(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> crate::player::Owner {
        self.modes[modes[self.applying as usize]]
            .effect
            .as_ref()
            .unwrap()
            .priority(db, source, already_selected, modes)
    }

    fn wants_input(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> bool {
        self.modes[modes[self.applying as usize]]
            .effect
            .as_ref()
            .unwrap()
            .wants_input(db, source, already_selected, modes)
    }

    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> Options {
        self.modes[modes[self.applying as usize]]
            .effect
            .as_ref()
            .unwrap()
            .options(db, source, already_selected, modes)
    }

    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        match self.modes[selected.modes[self.applying as usize]]
            .effect
            .as_mut()
            .unwrap()
            .select(db, source, option, selected)
        {
            SelectionResult::Complete => {
                self.applying += 1;

                if (self.applying as usize) == selected.modes.len() {
                    SelectionResult::Complete
                } else {
                    SelectionResult::TryAgain
                }
            }
            s => s,
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut results = vec![];

        for mode in selected.modes.clone() {
            results.extend(self.modes[mode].effect.as_mut().unwrap().apply(
                db,
                source,
                selected,
                skip_replacement,
            ))
        }

        results
    }
}
