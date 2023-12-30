use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    effects::{Effect, EffectBehaviors, EffectDuration},
    in_play::{self, OnBattlefield},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct ExileTarget {
    duration: EffectDuration,
    restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::ExileTarget> for ExileTarget {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ExileTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            duration: value.duration.get_or_default().try_into()?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for ExileTarget {
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in in_play::all_cards(db) {
            if card.passes_restrictions(db, source, &source.restrictions(db))
                && card.is_in_location::<OnBattlefield>(db)
                && card.passes_restrictions(db, source, &self.restrictions)
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
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        for target in targets {
            results.push_settled(ActionResult::ExileTarget {
                source,
                target,
                duration: self.duration,
                reason: None,
            });
        }
    }
}
