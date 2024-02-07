use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{effects::TutorLibrary, targets::Location},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for TutorLibrary {
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
        source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        Options::MandatoryList(
            self.valid_targets(db, source, already_selected)
                .map(|card| card.name(db).clone())
                .enumerate()
                .collect_vec(),
        )
    }

    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if let Some(option) = option {
            let card = self
                .valid_targets(db, source, selected)
                .nth(option)
                .unwrap();
            self.selected.push(card.into());

            if self.selected.len() == self.targets.len() {
                SelectionResult::Complete
            } else {
                SelectionResult::PendingChoice
            }
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let mut results = vec![];

        for (card, dest) in self
            .selected
            .iter()
            .zip(
                self.targets
                    .iter_mut()
                    .map(|target| target.destination.as_mut().unwrap()),
            )
            .rev()
        {
            let card: CardId = card.clone().into();
            if self.reveal {
                db[card].revealed = true;
            }

            results.push(EffectBundle {
                push_on_enter: Some(vec![Selected {
                    location: Some(Location::IN_LIBRARY),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                }]),
                source,
                effects: vec![dest.clone().into()],
                ..Default::default()
            });
        }

        results
    }
}

impl TutorLibrary {
    fn valid_targets<'db>(
        &'db self,
        db: &'db Database,
        source: Option<CardId>,
        selected: &'db [Selected],
    ) -> impl Iterator<Item = CardId> + 'db {
        db.all_players[selected.first().unwrap().player().unwrap()]
            .library
            .cards
            .iter()
            .copied()
            .filter(move |card| {
                card.passes_restrictions(
                    db,
                    LogId::current(db),
                    source.unwrap(),
                    &self.targets[self.selected.len()].restrictions,
                ) && !self.selected.iter().any(|selected| selected == card)
            })
    }
}
