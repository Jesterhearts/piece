use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, Options, PendingEffects, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    log::{Log, LogId},
    protogen::effects::SelectTargets,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectTargets {
    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        Options::OptionalList(
            db.cards
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
                .collect_vec(),
        )
    }

    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
        _modes: &mut Vec<usize>,
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
        } else {
            SelectionResult::Complete
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        for target in selected.iter() {
            Log::card_chosen(db, target.id(db).unwrap());
        }
    }
}
