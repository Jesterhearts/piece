use std::vec::IntoIter;

use crate::{
    action_result::ActionResult,
    effects::{EffectBehaviors, ReplacementEffect},
    in_play::Database,
    log::LogId,
    pending_results::PendingResults,
    player::Player,
    protogen::effects::{controller_draws_cards::Count, ControllerDrawsCards},
};

impl EffectBehaviors for ControllerDrawsCards {
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
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let count = match self.count.as_ref().unwrap() {
            Count::Fixed(count) => count.count as usize,
            Count::NumberOfPermanentsMatching(matching) => db.battlefield[controller]
                .iter()
                .filter(|card| {
                    card.passes_restrictions(db, LogId::current(db), source, &matching.restrictions)
                })
                .count(),
        };

        results.push_settled(ActionResult::DrawCards {
            target: controller,
            count,
        });
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let count = match self.count.as_ref().unwrap() {
            Count::Fixed(count) => count.count as usize,
            Count::NumberOfPermanentsMatching(matching) => db.battlefield[controller]
                .iter()
                .filter(|card| {
                    card.passes_restrictions(db, LogId::current(db), source, &matching.restrictions)
                })
                .count(),
        };
        results.push_settled(ActionResult::DrawCards {
            target: controller,
            count,
        });
    }

    fn replace_draw(
        &self,
        db: &mut Database,
        player: crate::player::Owner,
        replacements: &mut IntoIter<(crate::in_play::CardId, ReplacementEffect)>,
        controller: crate::player::Controller,
        _count: usize,
        results: &mut PendingResults,
    ) {
        let count = match self.count.as_ref().unwrap() {
            Count::Fixed(count) => count.count as usize,
            Count::NumberOfPermanentsMatching(matching) => db.battlefield[controller]
                .iter()
                .filter(|card| {
                    card.passes_restrictions(db, LogId::current(db), **card, &matching.restrictions)
                })
                .count(),
        };

        Player::draw_with_replacement(db, player, replacements, count, results);
    }
}
