use crate::{
    effects::{EffectBehaviors, Options, PendingEffects, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    protogen::effects::PayCost,
    stack::Selected,
};

impl EffectBehaviors for PayCost {
    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> Options {
        self.cost
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
        modes: &mut Vec<usize>,
    ) -> SelectionResult {
        if !self.saved_selected {
            selected.save();
            self.saved_selected = true;
        }

        self.cost
            .as_mut()
            .unwrap()
            .select(db, source, option, selected, modes)
    }

    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        modes: &[usize],
        skip_replacement: bool,
    ) {
        self.cost
            .as_mut()
            .unwrap()
            .apply(db, pending, source, selected, modes, skip_replacement);

        if self.saved_selected {
            let _ = selected.restore();
            self.saved_selected = false;
        }
    }
}
