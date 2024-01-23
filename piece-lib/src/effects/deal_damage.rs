use std::collections::HashSet;

use crate::{
    action_result::{damage_target::DamageTarget, ActionResult},
    effects::EffectBehaviors,
    log::LogId,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::{
        effects::{effect::Effect, DealDamage},
        types::Type,
    },
    stack::ActiveTarget,
    types::TypeSet,
};

impl EffectBehaviors for DealDamage {
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
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in db.battlefield.battlefields.values().flat_map(|b| b.iter()) {
            if card.passes_restrictions(
                db,
                log_session,
                source,
                &source.faceup_face(db).restrictions,
            ) && card.types_intersect(db, &TypeSet::from([Type::CREATURE]))
                && card.can_be_targeted(db, controller)
                && card.passes_restrictions(db, log_session, source, &self.restrictions)
            {
                let target = ActiveTarget::Battlefield { id: *card };
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
            }
        }

        for player in db.all_players.all_players() {
            // TODO player hexproof, non-all-target-damage
            if player.passes_restrictions(
                db,
                log_session,
                controller,
                &source.faceup_face(db).restrictions,
            ) && player.passes_restrictions(db, log_session, controller, &self.restrictions)
            {
                targets.push(ActiveTarget::Player { id: player });
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
        let valid = self
            .valid_targets(
                db,
                source,
                LogId::current(db),
                controller,
                &HashSet::default(),
            )
            .into_iter()
            .collect::<HashSet<_>>();

        for target in targets {
            if valid.contains(&target) {
                results.push_settled(ActionResult::from(DamageTarget {
                    quantity: self.quantity,
                    target,
                }));
            }
        }
    }
}
