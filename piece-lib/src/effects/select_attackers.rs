use itertools::Itertools;

use crate::{
    effects::{
        EffectBehaviors, EffectBundle, Options, PendingEffects, SelectedStack, SelectionResult,
    },
    in_play::{CardId, Database},
    protogen::{
        effects::{DeclareAttacking, Effect, SelectAttackers},
        targets::Location,
    },
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectAttackers {
    fn options(
        &self,
        db: &Database,
        _source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        if self.attackers.len() == self.targets.len() {
            Options::OptionalList(
                self.valid_attackers(db, already_selected)
                    .map(|card| card.name(db).clone())
                    .enumerate()
                    .collect_vec(),
            )
        } else {
            Options::MandatoryList(
                already_selected
                    .iter()
                    .filter_map(|selected| selected.player())
                    .map(|player| db.all_players[player].name.clone())
                    .enumerate()
                    .collect_vec(),
            )
        }
    }

    fn select(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
        _modes: &mut Vec<usize>,
    ) -> SelectionResult {
        if let Some(option) = option {
            if selected.is_empty() {
                return SelectionResult::Complete;
            }

            if self.attackers.len() == self.targets.len() {
                let attacker = self
                    .valid_attackers(db, selected)
                    .nth(option)
                    .unwrap()
                    .into();

                self.attackers.push(attacker);
            } else {
                self.targets.push(
                    selected
                        .iter()
                        .filter_map(|selected| selected.player())
                        .nth(option)
                        .unwrap()
                        .into(),
                );
            }

            SelectionResult::PendingChoice
        } else if self.attackers.len() == self.targets.len() {
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        _db: &mut Database,
        pending: &mut PendingEffects,
        _source: Option<CardId>,
        _selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let mut selected = SelectedStack::new(
            self.targets
                .iter()
                .map(|target| Selected {
                    location: None,
                    target_type: TargetType::Player(target.clone().into()),
                    targeted: false,
                    restrictions: vec![],
                })
                .collect_vec(),
        );

        selected.save();
        selected.clear();
        selected.extend(self.attackers.iter().map(|attacker| Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(attacker.clone().into()),
            targeted: false,
            restrictions: vec![],
        }));

        pending.push_back(EffectBundle {
            selected,
            effects: vec![Effect {
                effect: Some(DeclareAttacking::default().into()),
                ..Default::default()
            }],
            ..Default::default()
        });
    }
}

impl SelectAttackers {
    fn valid_attackers<'db>(
        &'db self,
        db: &'db Database,
        selected: &'db [Selected],
    ) -> impl Iterator<Item = CardId> + 'db {
        selected
            .iter()
            .filter_map(|selected| selected.id(db))
            .filter(|selected| !self.attackers.iter().any(|card| *selected == *card))
    }
}