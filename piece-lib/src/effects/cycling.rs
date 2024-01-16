use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::{
        effects::{effect::Effect, Cycling},
        targets::{restriction, Restriction},
    },
    stack::ActiveTarget,
};

impl EffectBehaviors for Cycling {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        if !self.types.is_empty() || !self.subtypes.is_empty() {
            1
        } else {
            0
        }
    }

    fn cycling(&self) -> bool {
        true
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        controller: crate::player::Controller,
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        if self.types.is_empty() && self.subtypes.is_empty() {
            return vec![];
        }

        let restrictions = [Restriction {
            restriction: Some(restriction::Restriction::from(restriction::OfType {
                types: self.types.clone(),
                subtypes: self.subtypes.clone(),
                ..Default::default()
            })),
            ..Default::default()
        }];

        db.all_players[controller]
            .library
            .cards
            .iter()
            .filter(|card| card.passes_restrictions(db, log_session, source, &restrictions))
            .map(|card| ActiveTarget::Library { id: *card })
            .collect_vec()
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if self.types.is_empty() && self.subtypes.is_empty() {
            results.push_settled(ActionResult::DrawCards {
                target: controller,
                count: 1,
            })
        } else {
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
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if self.types.is_empty() && self.subtypes.is_empty() {
            results.push_settled(ActionResult::DrawCards {
                target: controller,
                count: 1,
            });
        } else {
            for target in targets {
                let ActiveTarget::Library { id } = target else {
                    unreachable!()
                };

                results.push_settled(ActionResult::RevealCard(id));
                results.push_settled(ActionResult::MoveToHandFromLibrary(id));
            }
        }
    }
}
