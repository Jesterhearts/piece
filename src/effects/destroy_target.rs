use itertools::Itertools;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    effects::{Effect, EffectBehaviors},
    in_play::{self, target_from_location, OnBattlefield},
    protogen,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct DestroyTarget {
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::DestroyTarget> for DestroyTarget {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::DestroyTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for DestroyTarget {
    fn needs_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &mut in_play::Database,
        source: in_play::CardId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in in_play::cards::<OnBattlefield>(db) {
            if card.passes_restrictions(db, source, &source.restrictions(db))
                && card.can_be_targeted(db, controller)
                && card.passes_restrictions(db, source, &self.restrictions)
                && !card.indestructible(db)
            {
                let target = target_from_location(db, card);
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
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
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::DestroyTarget(
            targets.into_iter().exactly_one().unwrap(),
        ))
    }
}
