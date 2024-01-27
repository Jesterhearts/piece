use itertools::Itertools;

use crate::{
    effects::{
        ApplyResult, EffectBehaviors, Options, SelectedStack, SelectionResult,
    },
    in_play::{CardId, Database},
    protogen::effects::{Dest, MoveToBottomOfLibrary, MoveToTopOfLibrary, Scry},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for Scry {
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

        if self.placing == 0 {
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
        _modes: &mut Vec<usize>,
    ) -> SelectionResult {
        if let Some(option) = option {
            if self.dests.len() == self.placing as usize {
                self.dests.push(if self.placing == 0 {
                    Dest {
                        count: u32::MAX,
                        destination: Some(MoveToBottomOfLibrary::default().into()),
                        ..Default::default()
                    }
                } else {
                    Dest {
                        count: u32::MAX,
                        destination: Some(MoveToTopOfLibrary::default().into()),
                        ..Default::default()
                    }
                });
            }
            let dest = &mut self.dests[self.placing as usize];
            let card = selected.remove(option);

            dest.cards.push(card.id(db).unwrap().into());

            if selected.is_empty() {
                SelectionResult::Complete
            } else {
                SelectionResult::PendingChoice
            }
        } else if self.placing == 0 {
            self.placing += 1;
            SelectionResult::PendingChoice
        } else {
            for card in selected.drain(..) {
                self.dests[self.placing as usize]
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
        modes: &[usize],
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut pending = vec![];

        for dest in self.dests.iter_mut() {
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
                    modes,
                    skip_replacement,
                ));
            }
        }

        pending
    }
}
