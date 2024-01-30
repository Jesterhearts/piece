use itertools::Itertools;

use crate::{
    effects::{ApplyResult, EffectBehaviors, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    protogen::{effects::Cycling, targets::Location},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for Cycling {
    fn wants_input(
        &self,
        _db: &Database,
        _source: Option<CardId>,
        _already_selected: &[Selected],
        _modes: &[usize],
    ) -> bool {
        !self.types.is_empty() || !self.subtypes.is_empty()
    }

    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        _already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        if self.types.is_empty() && self.subtypes.is_empty() {
            Options::OptionalList(vec![])
        } else {
            Options::MandatoryList(
                self.valid_targets(db, source)
                    .map(|card| card.name(db).clone())
                    .enumerate()
                    .collect_vec(),
            )
        }
    }

    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if !self.types.is_empty() || !self.subtypes.is_empty() {
            let mut valid_targets = self.valid_targets(db, source);
            if let Some(option) = option {
                selected.save();
                selected.clear();
                selected.push(Selected {
                    location: Some(Location::IN_LIBRARY),
                    target_type: TargetType::Card(valid_targets.nth(option).unwrap()),
                    targeted: false,
                    restrictions: vec![],
                });

                SelectionResult::Complete
            } else if valid_targets.next().is_none() {
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
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        if !self.types.is_empty() || !self.subtypes.is_empty() {
            let tutoring = selected
                .restore()
                .into_iter()
                .exactly_one()
                .unwrap()
                .id(db)
                .unwrap();
            tutoring.move_to_hand(db);
        }

        vec![]
    }
}

impl Cycling {
    fn valid_targets<'db>(
        &'db self,
        db: &'db Database,
        source: Option<CardId>,
    ) -> impl Iterator<Item = CardId> + 'db {
        db.all_players[db[source.unwrap()].controller]
            .library
            .cards
            .iter()
            .copied()
            .filter(move |card| {
                card.types_intersect(db, &(&self.types).into())
                    && card.subtypes_intersect(db, &(&self.subtypes).into())
            })
    }
}
