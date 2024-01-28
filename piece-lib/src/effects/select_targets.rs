use itertools::Itertools;

use crate::{
    effects::{ApplyResult, EffectBehaviors, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    log::{Log, LogId},
    protogen::effects::SelectTargets,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectTargets {
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
        let options = db
            .cards
            .keys()
            .copied()
            .filter(|card| {
                card.passes_restrictions(
                    db,
                    LogId::current(db),
                    source.unwrap(),
                    &self.restrictions,
                ) && !already_selected
                    .iter()
                    .any(|selected| selected.id(db).unwrap() == *card)
            })
            .map(|card| card.name(db).clone())
            .enumerate()
            .collect_vec();

        if self.optional {
            Options::OptionalList(options)
        } else {
            Options::MandatoryList(options)
        }
    }

    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if let Some(option) = option {
            let card = db
                .cards
                .keys()
                .copied()
                .filter(|card| {
                    card.passes_restrictions(
                        db,
                        LogId::current(db),
                        source.unwrap(),
                        &self.restrictions,
                    ) && !selected
                        .iter()
                        .any(|selected| selected.id(db).unwrap() == *card)
                })
                .nth(option)
                .unwrap();

            selected.push(Selected {
                location: card.location(db),
                target_type: TargetType::Card(card),
                targeted: true,
                restrictions: self.restrictions.clone(),
            });

            let count = self.count.count(db, source, selected);
            if selected.len() == (count as usize) {
                SelectionResult::Complete
            } else {
                SelectionResult::PendingChoice
            }
        } else if self.optional {
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        for target in selected.iter() {
            Log::card_chosen(db, target.id(db).unwrap());
        }

        vec![]
    }
}
