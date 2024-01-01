use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    effects::{BattlefieldModifier, Effect, EffectBehaviors, EffectDuration, ModifyBattlefield},
    in_play::{self, ModifierId, OnBattlefield},
    protogen,
    stack::ActiveTarget,
    targets::{ControllerRestriction, Restriction},
    types::Type,
};

#[derive(Debug, Clone)]
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
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn is_sorcery_speed(&self) -> bool {
        true
    }

    fn is_equip(&'static self) -> bool {
        true
    }

    fn valid_targets(
        &self,
        db: &mut in_play::Database,
        source: in_play::CardId,
        _controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in in_play::cards::<OnBattlefield>(db) {
            if card.passes_restrictions(db, source, &source.restrictions(db))
                && card.types_intersect(db, &IndexSet::from([Type::Creature]))
            {
                let target = ActiveTarget::Battlefield { id: card };
                if already_chosen.contains(&target) {
                    continue;
                }

                targets.push(target);
            }
        }

        targets
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());
        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect(self)),
            valid_targets,
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
        results: &mut crate::battlefield::PendingResults,
    ) {
        let target = targets.into_iter().exactly_one().unwrap();
        // This is a hack. I hope equipment doesn't come with anthem effects.
        // It probably works even so.
        source.deactivate_modifiers(db);
        source.activate_modifiers(db);
        for modifier in self.modifiers.iter() {
            let modifier = ModifierId::upload_temporary_modifier(
                db,
                source,
                &BattlefieldModifier {
                    modifier: modifier.clone(),
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                    restrictions: vec![
                        Restriction::Controller(ControllerRestriction::Self_),
                        Restriction::OfType {
                            types: IndexSet::from([Type::Creature]),
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
