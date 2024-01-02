use std::collections::HashSet;

use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    battlefield::{
        pay_costs::{PayCost, SpendMana},
        Pending, PendingResult,
    },
    effects::Effect,
    in_play::{CardId, Database},
    player::{mana_pool::SpendReason, AllPlayers, Controller},
    stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ChooseForEachPlayer {
    target_source: Effect,
    pub(crate) valid_targets: Vec<ActiveTarget>,
    chosen: IndexMap<Controller, usize>,
    card: CardId,
}

impl ChooseForEachPlayer {
    pub(crate) fn new(
        target_source: Effect,
        valid_targets: Vec<ActiveTarget>,
        card: CardId,
    ) -> Self {
        Self {
            target_source,
            valid_targets,
            chosen: Default::default(),
            card,
        }
    }

    pub(crate) fn compute_targets(
        &mut self,
        db: &mut Database,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> bool {
        let controller = self.card.controller(db);
        let new_targets =
            self.target_source
                .valid_targets(db, self.card, controller, already_chosen);
        if new_targets != self.valid_targets {
            self.valid_targets = new_targets;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub(crate) fn choose_targets(&mut self, db: &mut Database, choice: Option<usize>) -> bool {
        debug!("choosing target: {:?}", choice);
        if let Some(choice) = choice {
            if self.valid_targets.is_empty() {
                true
            } else if choice >= self.valid_targets.len() {
                false
            } else {
                *self
                    .chosen
                    .entry(self.valid_targets[choice].id().unwrap().controller(db))
                    .or_default() = choice;
                true
            }
        } else if self.valid_targets.len() == 1 {
            debug!("Choosing default only target");
            *self
                .chosen
                .entry(self.valid_targets[0].id().unwrap().controller(db))
                .or_default() = 0;
            true
        } else {
            false
        }
    }

    pub(crate) fn effect(&self) -> Effect {
        self.target_source.clone()
    }

    pub(crate) fn chosen_targets(&self) -> Vec<ActiveTarget> {
        let mut results = vec![];
        for choice in self.chosen.values() {
            results.push(self.valid_targets[*choice]);
        }

        results
    }

    pub(crate) fn chosen_targets_count(&self) -> usize {
        self.chosen.values().sum()
    }

    pub(crate) fn choices_complete(&self, db: &mut Database) -> bool {
        self.chosen_targets_count() >= self.target_source.wants_targets(db, self.card)
            || self.chosen_targets_count() >= self.valid_targets.len()
    }
}

impl PendingResult for ChooseForEachPlayer {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        self.valid_targets.len() <= 1
    }

    fn options(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        self.valid_targets
            .iter()
            .enumerate()
            .map(|(idx, target)| (idx, target.display(db, all_players)))
            .collect_vec()
    }

    fn description(&self, _db: &Database) -> String {
        "targets".to_string()
    }

    fn is_empty(&self) -> bool {
        self.valid_targets.is_empty()
    }

    fn make_choice(
        &mut self,
        db: &mut Database,
        _all_players: &mut AllPlayers,
        choice: Option<usize>,
        results: &mut super::PendingResults,
    ) -> bool {
        if self.choose_targets(db, choice) {
            if self.choices_complete(db) {
                let choices = self.chosen_targets();

                for target in choices.iter() {
                    if let ActiveTarget::Battlefield { id } = target {
                        if let Some(ward) = id.ward(db) {
                            results.push_pay_costs(PayCost::SpendMana(SpendMana::new(
                                ward.mana_cost.clone(),
                                self.card,
                                SpendReason::Other,
                            )));
                        }
                    }
                }

                results.all_chosen_targets.extend(choices.iter().copied());
                if results.add_to_stack.is_none() {
                    let player = self.card.controller(db);
                    self.target_source.push_behavior_with_targets(
                        db,
                        choices.clone(),
                        false,
                        self.card,
                        player,
                        results,
                    );
                } else {
                    results.chosen_targets.push(choices.clone());
                }

                if !self.card.apply_individually(db) {
                    let player = self.card.controller(db);

                    let mut effect_or_auras = vec![];
                    results.pending.retain(|p| {
                        let Pending::ChooseForEachPlayer(choice) = p else {
                            return true;
                        };
                        effect_or_auras.push(choice.effect());
                        false
                    });

                    for effect in effect_or_auras {
                        if results.add_to_stack.is_some() {
                            results.chosen_targets.push(choices.clone());
                        } else {
                            effect.push_behavior_with_targets(
                                db,
                                choices.clone(),
                                false,
                                self.card,
                                player,
                                results,
                            );
                        }
                    }
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn recompute_targets(
        &mut self,
        db: &mut Database,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> bool {
        self.compute_targets(db, already_chosen)
    }
}
