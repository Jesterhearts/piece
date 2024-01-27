use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, Options, PendingEffects, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    player::Controller,
    protogen::{
        effects::{pay_cost::ExileCardsSharingType, Duration},
        targets::Location,
    },
    stack::{Selected, TargetType},
    types::TypeSet,
};

impl EffectBehaviors for ExileCardsSharingType {
    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        let card_types = self
            .selected
            .iter()
            .map(|selected| &db[CardId::from(selected.clone())].modified_types)
            .collect_vec();

        let controller = db[source.unwrap()].controller;
        Options::MandatoryList(
            compute_targets(db, controller, card_types, already_selected)
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
            let card_types = self
                .selected
                .iter()
                .map(|selected| &db[CardId::from(selected.clone())].modified_types)
                .collect_vec();
            let controller = db[source.unwrap()].controller;

            let card = compute_targets(db, controller, card_types, selected)
                .nth(option)
                .unwrap();

            selected.push(Selected {
                location: card.location(db),
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: vec![],
            });
            self.selected.push(card.into());

            if self.selected.len() == (self.count as usize) {
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

fn compute_targets<'db>(
    db: &'db Database,
    controller: Controller,
    card_types: Vec<&'db TypeSet>,
    selected: &'db [Selected],
) -> impl Iterator<Item = CardId> + 'db {
    db.cards.keys().copied().filter(move |card| {
        db[*card].controller == controller
            && matches!(
                card.location(db),
                Some(Location::IN_GRAVEYARD | Location::ON_BATTLEFIELD)
            )
            && card_types
                .iter()
                .all(|types| card.types_intersect(db, types))
            && !selected
                .iter()
                .any(|selected| selected.id(db).unwrap() == *card)
    })
}
