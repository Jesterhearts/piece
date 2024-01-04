use std::collections::HashSet;

use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    battlefield::{
        pay_costs::{PayCost, SpendMana},
        ActionResult, Pending, PendingResult, TargetSource,
    },
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    player::{mana_pool::SpendReason, AllPlayers},
    stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ChooseTargets {
    target_source: TargetSource,
    pub(crate) valid_targets: Vec<ActiveTarget>,
    chosen: IndexMap<usize, usize>,
    skipping_remainder: bool,
    card: CardId,
}

impl ChooseTargets {
    pub(crate) fn new(
        target_source: TargetSource,
        valid_targets: Vec<ActiveTarget>,
        card: CardId,
    ) -> Self {
        Self {
            target_source,
            valid_targets,
            chosen: Default::default(),
            skipping_remainder: false,
            card,
        }
    }

    pub(crate) fn compute_targets(
        &mut self,
        db: &mut Database,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> bool {
        let controller = self.card.controller(db);
        match &self.target_source {
            TargetSource::Effect(effect) => {
                let new_targets = effect.valid_targets(db, self.card, controller, already_chosen);
                if new_targets != self.valid_targets {
                    self.valid_targets = new_targets;
                    true
                } else {
                    false
                }
            }
            TargetSource::Aura(_) => {
                let new_targets = self.card.targets_for_aura(db).unwrap();
                if new_targets != self.valid_targets {
                    self.valid_targets = new_targets;
                    true
                } else {
                    false
                }
            }
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
                *self.chosen.entry(choice).or_default() += 1;
                true
            }
        } else if self.valid_targets.len() == 1 {
            debug!("Choosing default only target");
            *self.chosen.entry(0).or_default() += 1;
            true
        } else if self.can_skip(db) {
            self.skipping_remainder = true;
            true
        } else {
            false
        }
    }

    pub(crate) fn chosen_targets_and_effect(&self) -> (Vec<ActiveTarget>, TargetSource) {
        let mut results = vec![];
        for choice in self
            .chosen
            .iter()
            .flat_map(|(choice, count)| std::iter::repeat(*choice).take(*count))
        {
            results.push(self.valid_targets[choice]);
        }

        (results, self.target_source.clone())
    }

    fn effect_or_aura(&self) -> TargetSource {
        self.target_source.clone()
    }

    pub(crate) fn chosen_targets_count(&self) -> usize {
        self.chosen.values().sum()
    }

    pub(crate) fn choices_complete(&self, db: &mut Database) -> bool {
        self.chosen_targets_count() >= self.target_source.wants_targets(db, self.card)
            || self.chosen_targets_count() >= self.valid_targets.len()
            || (self.can_skip(db) && self.skipping_remainder)
    }

    pub(crate) fn can_skip(&self, db: &mut Database) -> bool {
        self.chosen_targets_count() >= self.target_source.needs_targets(db, self.card)
            || self.chosen_targets_count() >= self.valid_targets.len()
    }
}

impl PendingResult for ChooseTargets {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        self.valid_targets.is_empty()
    }

    fn options(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        self.valid_targets
            .iter()
            .enumerate()
            .map(|(idx, target)| (idx, target.display(db, all_players)))
            .collect_vec()
    }

    fn description(&self, db: &Database) -> String {
        format!("targets for {}", self.card.name(db))
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
                let (choices, effect_or_aura) = self.chosen_targets_and_effect();

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
                    match effect_or_aura {
                        TargetSource::Effect(effect) => {
                            effect.push_behavior_with_targets(
                                db,
                                choices.clone(),
                                false,
                                self.card,
                                player,
                                results,
                            );
                        }
                        TargetSource::Aura(aura) => {
                            results.push_settled(ActionResult::ApplyAuraToTarget {
                                aura,
                                target: *choices.iter().exactly_one().unwrap(),
                            });
                        }
                    }
                } else {
                    results.chosen_targets.push(choices.clone());
                }

                if !self.card.apply_individually(db) {
                    let player = self.card.controller(db);

                    let mut effect_or_auras = vec![];
                    results.pending.retain(|p| {
                        let Pending::ChooseTargets(choice) = p else {
                            return true;
                        };
                        effect_or_auras.push(choice.effect_or_aura());
                        false
                    });

                    for effect_or_aura in effect_or_auras {
                        if results.add_to_stack.is_some() {
                            results.chosen_targets.push(choices.clone());
                        } else {
                            match effect_or_aura {
                                TargetSource::Effect(effect) => {
                                    effect.push_behavior_with_targets(
                                        db,
                                        choices.clone(),
                                        false,
                                        self.card,
                                        player,
                                        results,
                                    );
                                }
                                TargetSource::Aura(aura) => {
                                    results.push_settled(ActionResult::ApplyAuraToTarget {
                                        aura,
                                        target: *choices.iter().exactly_one().unwrap(),
                                    })
                                }
                            }
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
