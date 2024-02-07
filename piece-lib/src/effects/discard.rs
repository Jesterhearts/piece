use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    effects::{
        ApplyResult, EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult,
    },
    in_play::{CardId, Database},
    log::Log,
    protogen::{
        effects::{Discard, MoveToGraveyard, PopSelected},
        targets::Location,
    },
    stack::{Selected, TargetType},
};

impl EffectBehaviors for Discard {
    fn wants_input(
        &self,
        db: &Database,
        source: Option<CardId>,
        selected: &[Selected],
        _modes: &[usize],
    ) -> bool {
        let count = self.count.count(db, source, selected);
        count != (self.cards.len() as i32)
    }

    fn options(
        &self,
        db: &Database,
        _source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        let in_hand = &db.hand[already_selected.first().unwrap().player().unwrap()];
        Options::MandatoryList(
            self.valid_targets(in_hand)
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
    ) -> super::SelectionResult {
        if let Some(option) = option {
            let in_hand = &db.hand[selected.first().unwrap().player().unwrap()];
            let card = self.valid_targets(in_hand).nth(option).unwrap();
            self.cards.push(card.into());

            let count = self.count.count(db, source, selected);
            if count == (self.cards.len() as i32) {
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
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        selected.save();
        selected.clear();
        selected.extend(
            self.cards
                .iter()
                .map(|card| card.clone().into())
                .map(|card| Selected {
                    location: Some(Location::IN_HAND),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                }),
        );

        for target in self.cards.iter().map(|card| card.clone().into()) {
            Log::discarded(db, target)
        }

        vec![ApplyResult::PushFront(EffectBundle {
            source,
            effects: vec![
                MoveToGraveyard::default().into(),
                PopSelected::default().into(),
            ],
            ..Default::default()
        })]
    }
}

impl Discard {
    fn valid_targets<'db>(
        &'db self,
        in_hand: &'db IndexSet<CardId>,
    ) -> impl Iterator<Item = CardId> + 'db {
        in_hand
            .iter()
            .copied()
            .filter(|card| !self.cards.iter().any(|selected| card == selected))
    }
}
