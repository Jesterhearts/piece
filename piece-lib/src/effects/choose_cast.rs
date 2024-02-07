use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    protogen::effects::{CastSelected, ChooseCast, MoveToHand, PopSelected},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for ChooseCast {
    fn wants_input(
        &self,
        _db: &Database,
        _source: Option<CardId>,
        selected: &[Selected],
        _modes: &[usize],
    ) -> bool {
        self.chosen.len() != selected.len()
    }

    fn options(
        &self,
        db: &Database,
        _source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        Options::OptionalList(
            already_selected
                .iter()
                .map(|target| target.id(db).unwrap())
                .filter(|id| !self.chosen.iter().any(|card| card == id))
                .map(|card| card.name(db).clone())
                .enumerate()
                .collect_vec(),
        )
    }

    fn select(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if let Some(option) = option {
            let option = selected
                .iter()
                .map(|target| target.id(db).unwrap())
                .filter(|id| !self.chosen.iter().any(|card| card == id))
                .nth(option)
                .unwrap();

            self.chosen.push(option.into());
            if self.chosen.len() == selected.len() {
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
    ) -> Vec<EffectBundle> {
        let mut results = vec![];
        if self.discovering {
            let not_cast = selected
                .iter()
                .filter(|target| {
                    !self
                        .chosen
                        .iter()
                        .any(|card| *card == target.id(db).unwrap())
                })
                .cloned()
                .collect_vec();
            results.push(EffectBundle {
                push_on_enter: Some(not_cast),
                effects: vec![MoveToHand::default().into(), PopSelected::default().into()],
                ..Default::default()
            });
        }

        for card in self.chosen.iter().rev() {
            let card: CardId = card.clone().into();
            results.push(EffectBundle {
                push_on_enter: Some(vec![Selected {
                    location: card.location(db),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                }]),
                effects: vec![
                    CastSelected {
                        pay_costs: self.pay_costs,
                        ..Default::default()
                    }
                    .into(),
                    PopSelected::default().into(),
                ],
                ..Default::default()
            });
        }

        results
    }
}
