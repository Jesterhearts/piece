use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::{target_from_location, Database, ModifierId},
    pending_results::{choose_targets::ChooseTargets, PendingResults, TargetSource},
    player::Controller,
    protogen::effects::{effect::Effect, BattlefieldModifier, Duration, ModifyTarget},
    stack::ActiveTarget,
};

impl EffectBehaviors for ModifyTarget {
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

    fn valid_targets(
        &self,
        db: &Database,
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        controller: Controller,
        already_chosen: &std::collections::HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        let mut targets = vec![];
        for card in db.cards.keys() {
            if card.can_be_targeted(db, controller)
                && card.passes_restrictions(
                    db,
                    log_session,
                    source,
                    &source.faceup_face(db).restrictions,
                )
                && card.passes_restrictions(db, log_session, source, &self.restrictions)
            {
                let target = target_from_location(db, *card);
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
            }
        }

        targets
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: crate::in_play::CardId,
        controller: Controller,
        results: &mut PendingResults,
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
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        let mut final_targets = vec![];
        for target in targets {
            match target {
                ActiveTarget::Battlefield { .. } => {
                    final_targets.push(target);
                }
                ActiveTarget::Graveyard { .. } => {
                    final_targets.push(target);
                }
                _ => unreachable!(),
            }
        }

        let modifier = match self.duration.enum_value().unwrap() {
            Duration::UNTIL_TARGET_LEAVES_BATTLEFIELD => ModifierId::upload_temporary_modifier(
                db,
                final_targets.iter().exactly_one().unwrap().id(db).unwrap(),
                BattlefieldModifier {
                    modifier: self.modifier.clone(),
                    duration: self.duration,
                    ..Default::default()
                },
            ),
            _ => ModifierId::upload_temporary_modifier(
                db,
                source,
                BattlefieldModifier {
                    modifier: self.modifier.clone(),
                    duration: self.duration,
                    ..Default::default()
                },
            ),
        };

        results.push_settled(ActionResult::ModifyCreatures {
            targets: final_targets,
            modifier,
        });
    }
}
