use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::{Effect, EffectBehaviors},
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::{self, targets::Restriction},
    stack::ActiveTarget,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetToTopOfLibrary {
    restrictions: Vec<Restriction>,
    under_cards: usize,
}

impl TryFrom<&protogen::effects::TargetToTopOfLibrary> for TargetToTopOfLibrary {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::TargetToTopOfLibrary) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value.restrictions.clone(),
            under_cards: usize::try_from(value.under_cards)?,
        })
    }
}

impl EffectBehaviors for TargetToTopOfLibrary {
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
        for target in db
            .battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .filter(|card| {
                card.passes_restrictions(
                    db,
                    log_session,
                    source,
                    &source.faceup_face(db).restrictions,
                )
            })
            .collect_vec()
        {
            if target.can_be_targeted(db, controller)
                && target.passes_restrictions(db, log_session, source, &self.restrictions)
            {
                let target = ActiveTarget::Battlefield { id: *target };
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
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
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        for target in targets {
            results.push_settled(ActionResult::ReturnFromBattlefieldToLibrary {
                target,
                under_cards: self.under_cards,
            });
        }
    }
}