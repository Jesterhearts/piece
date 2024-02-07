use itertools::Itertools;

use crate::{
    effects::{ApplyResult, EffectBehaviors, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{
        cost::XIs,
        effects::{pay_cost::ExilePermanentsCmcX, Duration},
    },
    stack::{Selected, TargetType},
};

impl EffectBehaviors for ExilePermanentsCmcX {
    fn wants_input(
        &self,
        db: &Database,
        _source: Option<CardId>,
        selected: &[Selected],
        _modes: &[usize],
    ) -> bool {
        let exiled = self
            .selected
            .iter()
            .map(|card| db[CardId::from(card.clone())].modified_cost.cmc())
            .sum::<usize>();

        let x_is = match self.x_is.enum_value().unwrap() {
            XIs::MANA_VALUE_OF_SELECTED => db[selected.first().unwrap().id(db).unwrap()]
                .modified_cost
                .cmc(),
        };
        exiled < x_is
    }

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

        let exiled = self
            .selected
            .iter()
            .map(|card| db[CardId::from(card.clone())].modified_cost.cmc())
            .sum::<usize>();

        let x_is = match self.x_is.enum_value().unwrap() {
            XIs::MANA_VALUE_OF_SELECTED => db[already_selected.first().unwrap().id(db).unwrap()]
                .modified_cost
                .cmc(),
        };
        if exiled >= x_is {
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

        let exiled = self
            .selected
            .iter()
            .map(|card| db[CardId::from(card.clone())].modified_cost.cmc())
            .sum::<usize>();

        let x_is = match self.x_is.enum_value().unwrap() {
            XIs::MANA_VALUE_OF_SELECTED => db[selected.first().unwrap().id(db).unwrap()]
                .modified_cost
                .cmc(),
        };
        if exiled >= x_is {
            SelectionResult::Complete
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
    ) -> Vec<ApplyResult> {
        for card in self.selected.iter() {
            let card: CardId = card.clone().into();
            card.move_to_exile(db, source.unwrap(), None, Duration::PERMANENTLY)
        }

        vec![]
    }
}

impl ExilePermanentsCmcX {
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
