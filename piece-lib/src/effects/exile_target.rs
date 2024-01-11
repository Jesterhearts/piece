use crate::{
    action_result::ActionResult,
    effects::{Effect, EffectBehaviors, EffectDuration},
    in_play::target_from_location,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::{self, targets::Restriction},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExileTarget {
    duration: EffectDuration,
    restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::ExileTarget> for ExileTarget {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ExileTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            duration: value.duration.get_or_default().try_into()?,
            restrictions: value.restrictions.clone(),
        })
    }
}

impl EffectBehaviors for ExileTarget {
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
        _controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in db.cards.keys() {
            if card.passes_restrictions(
                db,
                log_session,
                source,
                &source.faceup_face(db).restrictions,
            ) && card.passes_restrictions(db, log_session, source, &self.restrictions)
            {
                let target = target_from_location(db, *card);
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
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
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
