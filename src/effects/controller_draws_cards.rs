use std::vec::IntoIter;

use anyhow::anyhow;

use crate::{
    battlefield::{ActionResult, PendingResults},
    effects::EffectBehaviors,
    in_play::{cards, Database, OnBattlefield, ReplacementEffectId},
    player::Player,
    protogen,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) enum Count {
    Fixed(usize),
    NumberOfPermanentsMatching(Vec<Restriction>),
}

impl TryFrom<&protogen::effects::controller_draw_cards::Count> for Count {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::controller_draw_cards::Count,
    ) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::controller_draw_cards::Count::Fixed(count) => {
                Ok(Self::Fixed(usize::try_from(count.count)?))
            }
            protogen::effects::controller_draw_cards::Count::NumberOfPermanentsMatching(
                matching,
            ) => Ok(Self::NumberOfPermanentsMatching(
                matching
                    .restrictions
                    .iter()
                    .map(Restriction::try_from)
                    .collect::<anyhow::Result<_>>()?,
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ControllerDrawsCards {
    count: Count,
}

impl TryFrom<&protogen::effects::ControllerDrawCards> for ControllerDrawsCards {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ControllerDrawCards) -> Result<Self, Self::Error> {
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
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let count = match &self.count {
            Count::Fixed(count) => *count,
            Count::NumberOfPermanentsMatching(matching) => cards::<OnBattlefield>(db)
                .into_iter()
                .filter(|card| card.passes_restrictions(db, source, matching))
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
        results: &mut crate::battlefield::PendingResults,
    ) {
        let count = match &self.count {
            Count::Fixed(count) => *count,
            Count::NumberOfPermanentsMatching(matching) => cards::<OnBattlefield>(db)
                .into_iter()
                .filter(|card| card.passes_restrictions(db, source, matching))
                .count(),
        };
        results.push_settled(ActionResult::DrawCards {
            target: controller,
            count,
        });
    }

    fn replace_draw(
        &self,
        player: &mut Player,
        db: &mut Database,
        replacements: &mut IntoIter<ReplacementEffectId>,
        _controller: crate::player::Controller,
        _count: usize,
        results: &mut PendingResults,
    ) {
        let count = match &self.count {
            Count::Fixed(count) => *count,
            Count::NumberOfPermanentsMatching(matching) => cards::<OnBattlefield>(db)
                .into_iter()
                .filter(|card| card.passes_restrictions(db, *card, matching))
                .count(),
        };

        player.draw_with_replacement(db, replacements, count, results);
    }
}
