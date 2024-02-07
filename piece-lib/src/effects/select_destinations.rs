use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    protogen::effects::SelectDestinations,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectDestinations {
    fn wants_input(
        &self,
        _db: &Database,
        _source: Option<CardId>,
        _already_selected: &[Selected],
        _modes: &[usize],
    ) -> bool {
        (self.placing as usize) < self.destinations.len()
    }

    fn options(
        &self,
        db: &Database,
        _source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        let options = already_selected
            .iter()
            .map(|option| option.display(db))
            .enumerate()
            .collect_vec();

        if (self.placing as usize) < self.destinations.len() - 1 {
            Options::OptionalList(options)
        } else {
            Options::ListWithDefault(options)
        }
    }

    fn select(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if let Some(option) = option {
            let dest = &mut self.destinations[self.placing as usize];
            let card = selected.remove(option);

            dest.cards.push(card.id(db).unwrap().into());

            if dest.cards.len() == (dest.count as usize) {
                self.placing += 1;
            }

            if selected.is_empty() {
                SelectionResult::Complete
            } else {
                SelectionResult::PendingChoice
            }
        } else if (self.placing as usize) < self.destinations.len().saturating_sub(2) {
            self.placing += 1;
            SelectionResult::PendingChoice
        } else {
            for card in selected
                .drain(..)
                .take(self.destinations[self.placing as usize].count as usize)
            {
                self.destinations[self.placing as usize]
                    .cards
                    .push(card.id(db).unwrap().into());
            }

            SelectionResult::Complete
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let mut pending = vec![];
        for dest in self.destinations.iter_mut() {
            for card in dest.cards.iter() {
                let mut selected = SelectedStack::new(vec![Selected {
                    location: None,
                    target_type: TargetType::Card(card.clone().into()),
                    targeted: false,
                    restrictions: vec![],
                }]);

                pending.extend(dest.destination.as_mut().unwrap().apply(
                    db,
                    source,
                    &mut selected,
                    skip_replacement,
                ));
            }
        }

        pending
    }
}
