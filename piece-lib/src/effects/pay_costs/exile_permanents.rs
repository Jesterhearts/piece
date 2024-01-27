use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, Options, PendingEffects, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    log::LogId,
    player::Controller,
    protogen::effects::{pay_cost::ExilePermanents, Duration},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for ExilePermanents {
    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        let controller = db[source.unwrap()].controller;
        let targets = self
            .compute_targets(db, controller, source, already_selected)
            .map(|card| card.name(db).clone())
            .enumerate()
            .collect_vec();

        if self.selected.len() < (self.minimum as usize) {
            Options::MandatoryList(targets)
        } else {
            Options::OptionalList(targets)
        }
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
            let controller = db[source.unwrap()].controller;
            let card = self
                .compute_targets(db, controller, source, selected)
                .nth(option)
                .unwrap();

            selected.push(Selected {
                location: card.location(db),
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: vec![],
            });
            self.selected.push(card.into());

            if self.selected.len() == (self.maximum as usize) {
                SelectionResult::Complete
            } else {
                SelectionResult::PendingChoice
            }
        } else if self.selected.len() >= (self.minimum as usize) {
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        for card in self.selected.iter() {
            let card: CardId = card.clone().into();
            card.move_to_exile(db, source.unwrap(), None, Duration::PERMANENTLY)
        }
    }
}

impl ExilePermanents {
    fn compute_targets<'db>(
        &'db self,
        db: &'db Database,
        controller: Controller,
        source: Option<CardId>,
        already_selected: &'db [Selected],
    ) -> impl Iterator<Item = CardId> + 'db {
        db.cards.keys().copied().filter(move |card| {
            db[*card].controller == controller
                && card.passes_restrictions(
                    db,
                    LogId::current(db),
                    source.unwrap(),
                    &self.restrictions,
                )
                && !already_selected
                    .iter()
                    .any(|selected| selected.id(db).unwrap() == *card)
        })
    }
}
