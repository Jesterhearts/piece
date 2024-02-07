use crate::{
    effects::{EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    protogen::effects::{pay_cost::Cost, PayCost},
    stack::Selected,
};

impl EffectBehaviors for PayCost {
    fn wants_input(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> bool {
        self.cost
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
    ) -> SelectionResult {
        if !self.saved_selected {
            selected.save();
            self.saved_selected = true;
        }

        self.cost
            .as_mut()
            .unwrap()
            .select(db, source, option, selected)
    }

    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let results = self
            .cost
            .as_mut()
            .unwrap()
            .apply(db, source, selected, skip_replacement);

        if self.saved_selected {
            let _ = selected.restore();
        }

        results
    }
}

impl<T: Into<Cost>> From<T> for PayCost {
    fn from(value: T) -> Self {
        Self {
            cost: Some(value.into()),
            ..Default::default()
        }
    }
}
