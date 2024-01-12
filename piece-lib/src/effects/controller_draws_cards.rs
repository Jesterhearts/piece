use std::vec::IntoIter;

use anyhow::anyhow;

use crate::{
    action_result::ActionResult,
    effects::{EffectBehaviors, ReplacementAbility},
    in_play::Database,
    log::LogId,
    pending_results::PendingResults,
    player::Player,
    protogen::{self, effects::NumberOfPermanentsMatching},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Count {
    Fixed(usize),
    NumberOfPermanentsMatching(NumberOfPermanentsMatching),
}

impl TryFrom<&protogen::effects::controller_draws_cards::Count> for Count {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::controller_draws_cards::Count,
    ) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::controller_draws_cards::Count::Fixed(count) => {
                Ok(Self::Fixed(usize::try_from(count.count)?))
            }
            protogen::effects::controller_draws_cards::Count::NumberOfPermanentsMatching(
                matching,
            ) => Ok(Self::NumberOfPermanentsMatching(matching.clone())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControllerDrawsCards {
    count: Count,
}

impl TryFrom<&protogen::effects::ControllerDrawsCards> for ControllerDrawsCards {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ControllerDrawsCards) -> Result<Self, Self::Error> {
        Ok(Self {
            count: value
                .count
                .as_ref()
                .ok_or_else(|| anyhow!("Expected count to have a count set"))
                .and_then(Count::try_from)?,
        })
    }
}

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
        let count = match &self.count {
            Count::Fixed(count) => *count,
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
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let count = match &self.count {
            Count::Fixed(count) => *count,
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
        replacements: &mut IntoIter<(crate::in_play::CardId, ReplacementAbility)>,
        controller: crate::player::Controller,
        _count: usize,
        results: &mut PendingResults,
    ) {
        let count = match &self.count {
            Count::Fixed(count) => *count,
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
