use itertools::Itertools;

use crate::{
    effects::{
        EffectBehaviors, EffectBundle, Options, PendingEffects, SelectedStack, SelectionResult,
    },
    in_play::{CardId, Database},
    log::LogId,
    player::Controller,
    protogen::effects::{pay_cost::SacrificePermanent, Effect, MoveToGraveyard},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SacrificePermanent {
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

        Options::MandatoryList(targets)
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

            self.selected = protobuf::MessageField::some(card.into());

            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        effects: &mut PendingEffects,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let _ = _selected;
        let card: CardId = self.selected.as_ref().cloned().unwrap().into();
        effects.push_front(EffectBundle {
            selected: SelectedStack::new(vec![Selected {
                location: card.location(db),
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: vec![],
            }]),
            effects: vec![Effect {
                effect: Some(MoveToGraveyard::default().into()),
                ..Default::default()
            }],
            source,
            ..Default::default()
        });
    }
}

impl SacrificePermanent {
    fn compute_targets<'db>(
        &'db self,
        db: &'db Database,
        controller: Controller,
        source: Option<CardId>,
        already_selected: &'db [Selected],
    ) -> impl Iterator<Item = CardId> + 'db {
        db.battlefield[controller]
            .iter()
            .copied()
            .filter(move |card| {
                card.passes_restrictions(
                    db,
                    LogId::current(db),
                    source.unwrap(),
                    &self.restrictions,
                ) && !already_selected
                    .iter()
                    .any(|selected| selected.id(db).unwrap() == *card)
            })
    }
}
