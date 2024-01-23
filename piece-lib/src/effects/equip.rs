use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use protobuf::Enum;

use crate::{
    action_result::{modify_creatures::ModifyCreatures, ActionResult},
    effects::EffectBehaviors,
    in_play::ModifierId,
    log::LogId,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::{
        effects::{effect::Effect, BattlefieldModifier, Duration, Equip},
        empty::Empty,
        targets::{restriction, Restriction},
        types::Type,
    },
    stack::ActiveTarget,
    types::TypeSet,
};

impl EffectBehaviors for Equip {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn is_sorcery_speed(&self) -> bool {
        true
    }

    fn is_equip(&self) -> bool {
        true
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in db.battlefield[controller].iter() {
            if card.passes_restrictions(
                db,
                log_session,
                source,
                &source.faceup_face(db).restrictions,
            ) && card.types_intersect(db, &TypeSet::from([Type::CREATURE]))
            {
                let target = ActiveTarget::Battlefield { id: *card };
                if already_chosen.contains(&target) {
                    continue;
                }

                targets.push(target);
            }
        }

        targets
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let valid_targets = self.valid_targets(
            db,
            source,
            crate::log::LogId::current(db),
            controller,
            results.all_currently_targeted(),
        );
        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            crate::log::LogId::current(db),
            source,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let target = targets.into_iter().exactly_one().unwrap();

        if !self
            .valid_targets(
                db,
                source,
                LogId::current(db),
                controller,
                &HashSet::default(),
            )
            .into_iter()
            .any(|t| t == target)
        {
            return;
        }

        for modifier in db
            .modifiers
            .iter()
            .filter_map(|(id, modifier)| {
                if modifier.source == source {
                    Some(id)
                } else {
                    None
                }
            })
            .copied()
            .collect_vec()
        {
            db.modifiers.get_mut(&modifier).unwrap().modifying.clear();
        }

        for modifier in self.modifiers.iter() {
            let modifier = ModifierId::upload_temporary_modifier(
                db,
                source,
                BattlefieldModifier {
                    modifier: protobuf::MessageField::some(modifier.clone()),
                    duration: Duration::UNTIL_SOURCE_LEAVES_BATTLEFIELD.into(),
                    restrictions: vec![
                        Restriction {
                            restriction: Some(restriction::Restriction::from(
                                restriction::Controller {
                                    controller: Some(restriction::controller::Controller::Self_(
                                        Default::default(),
                                    )),
                                    ..Default::default()
                                },
                            )),
                            ..Default::default()
                        },
                        Restriction {
                            restriction: Some(restriction::Restriction::from(
                                restriction::OfType {
                                    types: HashMap::from([(
                                        Type::CREATURE.value(),
                                        Empty::default(),
                                    )]),
                                    ..Default::default()
                                },
                            )),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
            );

            results.push_settled(ActionResult::from(ModifyCreatures {
                targets: vec![target],
                modifier,
            }));
        }
    }
}
