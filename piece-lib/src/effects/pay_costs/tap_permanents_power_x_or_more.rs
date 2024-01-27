use itertools::Itertools;

use crate::{
    effects::{
        EffectBehaviors, EffectBundle, Options, PendingEffects, SelectedStack, SelectionResult,
    },
    in_play::{CardId, Database},
    log::LogId,
    protogen::{
        effects::{pay_cost::TapPermanentsPowerXOrMore, Effect, Tap},
        targets::Location,
    },
    stack::{Selected, TargetType},
};

impl EffectBehaviors for TapPermanentsPowerXOrMore {
    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        let controller = db[source.unwrap()].controller;
        let targets = self
            .select_targets(db, controller, source.unwrap(), already_selected)
            .map(|card| card.name(db).clone())
            .enumerate()
            .collect_vec();

        let tapped = self
            .selected
            .iter()
            .map(|card| CardId::from(card.clone()).power(db).unwrap_or_default())
            .sum::<i32>();
        if tapped >= (self.x_is as i32) {
            Options::OptionalList(targets)
        } else {
            Options::MandatoryList(targets)
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
                .select_targets(db, controller, source.unwrap(), selected)
                .nth(option)
                .unwrap();
            self.selected.push(card.into());
            selected.push(Selected {
                location: card.location(db),
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: vec![],
            });
        }

        let tapped = self
            .selected
            .iter()
            .map(|card| CardId::from(card.clone()).power(db).unwrap_or_default())
            .sum::<i32>();
        if tapped >= (self.x_is as i32) {
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        _db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        pending.push_front(EffectBundle {
            selected: SelectedStack::new(
                self.selected
                    .iter()
                    .map(|card| Selected {
                        location: Some(Location::ON_BATTLEFIELD),
                        target_type: TargetType::Card(card.clone().into()),
                        targeted: false,
                        restrictions: vec![],
                    })
                    .collect_vec(),
            ),
            effects: vec![Effect {
                effect: Some(Tap::default().into()),
                ..Default::default()
            }],
            source,
            ..Default::default()
        });
    }
}

impl TapPermanentsPowerXOrMore {
    fn select_targets<'db>(
        &'db self,
        db: &'db Database,
        controller: crate::player::Controller,
        source: CardId,
        already_selected: &'db [Selected],
    ) -> impl Iterator<Item = CardId> + 'db {
        db.battlefield[controller]
            .iter()
            .copied()
            .filter(move |card| {
                card.passes_restrictions(db, LogId::current(db), source, &self.restrictions)
                    && !already_selected
                        .iter()
                        .any(|selected| selected.id(db).unwrap() == *card)
            })
    }
}
