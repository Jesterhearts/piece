use std::collections::HashMap;

use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::{BattlefieldModifier, Effect, EffectBehaviors, EffectDuration, ModifyBattlefield},
    in_play::ModifierId,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::{self, empty::Empty, types::Type},
    stack::ActiveTarget,
    targets::{ControllerRestriction, Restriction},
    types::TypeSet,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Equip {
    modifiers: Vec<ModifyBattlefield>,
}

impl TryFrom<&protogen::effects::Equip> for Equip {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Equip) -> Result<Self, Self::Error> {
        Ok(Self {
            modifiers: value
                .modifiers
                .iter()
                .map(ModifyBattlefield::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

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
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let target = targets.into_iter().exactly_one().unwrap();
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
                    modifier: modifier.clone(),
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                    restrictions: vec![
                        Restriction::Controller(ControllerRestriction::Self_),
                        Restriction::OfType {
                            types: HashMap::from([(
                                Type::CREATURE.as_ref().to_string(),
                                Empty::default(),
                            )]),
                            subtypes: Default::default(),
                        },
                    ],
                },
            );

            results.push_settled(ActionResult::ModifyCreatures {
                targets: vec![target],
                modifier,
            });
        }
    }
}
