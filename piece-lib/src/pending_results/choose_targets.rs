use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    log::LogId,
    pending_results::{Pending, PendingResult, PendingResults, TargetSource},
    stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ChooseTargets {
    target_source: TargetSource,
    pub(crate) valid_targets: Vec<ActiveTarget>,
    chosen: Vec<ActiveTarget>,
    skipping_remainder: bool,
    log_session: LogId,
    card: CardId,
}

impl ChooseTargets {
    pub(crate) fn new(
        target_source: TargetSource,
        valid_targets: Vec<ActiveTarget>,
        log_session: LogId,
        card: CardId,
    ) -> Self {
        Self {
            target_source,
            valid_targets,
            chosen: Default::default(),
            skipping_remainder: false,
            log_session,
            card,
        }
    }

    pub(crate) fn compute_targets(
        &mut self,
        db: &mut Database,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> bool {
        let controller = db[self.card].controller;
        match &self.target_source {
            TargetSource::Effect(effect) => {
                let new_targets = effect.valid_targets(
                    db,
                    self.card,
                    self.log_session,
                    controller,
                    already_chosen,
                );
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
                self.chosen.push(self.valid_targets[choice]);
                true
            }
        } else if self.valid_targets.len() == 1 {
            debug!("Choosing default only target");
            self.chosen.push(self.valid_targets[0]);
            true
        } else if self.can_skip(db) {
            self.skipping_remainder = true;
            true
        } else {
            false
        }
    }

    pub(crate) fn chosen_targets_and_effect(&self) -> (Vec<ActiveTarget>, TargetSource) {
        (self.chosen.clone(), self.target_source.clone())
    }

    fn effect_or_aura(&self) -> TargetSource {
        self.target_source.clone()
    }

    pub(crate) fn chosen_targets_count(&self) -> usize {
        self.chosen.len()
    }

    pub(crate) fn choices_complete(&mut self, db: &mut Database, results: &PendingResults) -> bool {
        let _ = self.recompute_targets(db, results.all_currently_targeted());

        self.chosen_targets_count() >= self.target_source.wants_targets(db, self.card)
            || (self.chosen_targets_count() >= self.valid_targets.len()
                && self.chosen == self.valid_targets)
            || (self.can_skip(db) && self.skipping_remainder)
    }

    pub(crate) fn can_skip(&self, db: &mut Database) -> bool {
        self.chosen_targets_count() >= self.target_source.needs_targets(db, self.card)
            || self.chosen_targets_count() >= self.valid_targets.len()
    }
}

impl PendingResult for ChooseTargets {
    fn optional(&self, _db: &Database) -> bool {
        self.valid_targets.is_empty()
    }

    fn options(&self, db: &mut Database) -> Vec<(usize, String)> {
        self.valid_targets
            .iter()
            .enumerate()
            .map(|(idx, target)| (idx, target.display(db)))
            .collect_vec()
    }

    fn target_for_option(&self, _db: &Database, option: usize) -> Option<ActiveTarget> {
        self.valid_targets.get(option).copied()
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
        choice: Option<usize>,
        results: &mut super::PendingResults,
    ) -> bool {
        if self.choose_targets(db, choice) {
            results
                .all_chosen_targets
                .extend(self.chosen.iter().copied());

            if self.choices_complete(db, results) {
                let (choices, effect_or_aura) = self.chosen_targets_and_effect();

                if results.add_to_stack.is_empty() {
                    let player = db[self.card].controller;

                    match effect_or_aura {
                        TargetSource::Effect(effect) => {
                            effect.push_behavior_with_targets(
                                db,
                                choices.clone(),
                                self.card,
                                player,
                                results,
                            );
                        }
                        TargetSource::Aura(aura_source) => {
                            results.push_settled(ActionResult::ApplyAuraToTarget {
                                aura_source,
                                target: *choices.iter().exactly_one().unwrap(),
                            });
                        }
                    }
                } else {
                    results.chosen_targets.push(choices.clone());
                }

                if !self.card.faceup_face(db).apply_individually {
                    let player = db[self.card].controller;

                    let mut effect_or_auras = vec![];
                    results.pending.retain(|p| {
                        let Pending::ChooseTargets(choice) = p else {
                            return true;
                        };
                        effect_or_auras.push(choice.effect_or_aura());
                        false
                    });

                    for effect_or_aura in effect_or_auras {
                        if results.add_to_stack.is_empty() {
                            results.chosen_targets.push(choices.clone());
                        } else {
                            match effect_or_aura {
                                TargetSource::Effect(effect) => {
                                    effect.push_behavior_with_targets(
                                        db,
                                        choices.clone(),
                                        self.card,
                                        player,
                                        results,
                                    );
                                }
                                TargetSource::Aura(aura_source) => {
                                    results.push_settled(ActionResult::ApplyAuraToTarget {
                                        aura_source,
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
