use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    effects::EffectBehaviors,
    in_play::Database,
    log::LogId,
    pending_results::{choose_targets::ChooseTargets, PendingResults, TargetSource},
    protogen::ids::Controller,
    protogen::{
        effects::{effect::Effect, CopySpellOrAbility},
        ids::CardId,
    },
    stack::{ActiveTarget, Entry},
};

impl EffectBehaviors for CopySpellOrAbility {
    fn needs_targets(&self, _db: &Database, _source: &CardId) -> usize {
        1
    }

    fn wants_targets(&self, _db: &Database, _source: &CardId) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &Database,
        source: &CardId,
        log_session: LogId,
        _controller: &Controller,
        _already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        db.stack
            .entries
            .iter()
            .filter_map(|(id, entry)| {
                if entry.passes_restrictions(db, log_session, source, &self.restrictions) {
                    Some(ActiveTarget::Stack { id: id.clone() })
                } else {
                    None
                }
            })
            .collect_vec()
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: &CardId,
        controller: &Controller,
        results: &mut PendingResults,
    ) {
        let valid_targets = self.valid_targets(
            db,
            source,
            LogId::current(db),
            controller,
            results.all_currently_targeted(),
        );

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            crate::log::LogId::current(db),
            source.clone(),
        ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        _source: &CardId,
        controller: &Controller,
        results: &mut PendingResults,
    ) {
        for target in targets {
            let ActiveTarget::Stack { id } = target else {
                unreachable!()
            };

            match &db.stack.entries.get(&id).unwrap().ty {
                Entry::Card(source) => {
                    results.copy_card_to_stack(
                        source.clone(),
                        controller.clone(),
                        db.stack.entries.get(&id).unwrap().mode.clone(),
                        Some(db[source].x_is),
                    );

                    for effect in source.faceup_face(db).effects.iter() {
                        let valid_targets = effect.effect.as_ref().unwrap().valid_targets(
                            db,
                            source,
                            crate::log::LogId::current(db),
                            controller,
                            results.all_currently_targeted(),
                        );

                        if !valid_targets.is_empty() {
                            results.push_choose_targets(ChooseTargets::new(
                                TargetSource::Effect(effect.effect.as_ref().unwrap().clone()),
                                valid_targets,
                                crate::log::LogId::current(db),
                                source.clone(),
                            ));
                        }
                    }
                }
                Entry::Ability { source, ability } => {
                    results.copy_ability_to_stack(
                        source.clone(),
                        ability.clone(),
                        controller,
                        Some(db[source].x_is),
                    );

                    for effect in ability.effects(db) {
                        let effect = effect.effect.unwrap();
                        let valid_targets = effect.valid_targets(
                            db,
                            source,
                            crate::log::LogId::current(db),
                            controller,
                            results.all_currently_targeted(),
                        );

                        if !valid_targets.is_empty() {
                            results.push_choose_targets(ChooseTargets::new(
                                TargetSource::Effect(effect),
                                valid_targets,
                                crate::log::LogId::current(db),
                                source.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }
}
