use indexmap::IndexSet;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    controller::ControllerRestriction,
    effects::{Effect, EffectBehaviors, EffectDuration},
    in_play::{self, OnBattlefield},
    stack::ActiveTarget,
    types::Type,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExileTargetCreatureManifestTopOfLibrary;

impl EffectBehaviors for ExileTargetCreatureManifestTopOfLibrary {
    fn needs_targets(&self) -> usize {
        1
    }

    fn wants_targets(&self) -> usize {
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
            if card.passes_restrictions(
                db,
                source,
                ControllerRestriction::Any,
                &source.restrictions(db),
            ) && card.is_in_location::<OnBattlefield>(db)
                && card.types_intersect(db, &IndexSet::from([Type::Creature]))
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
        db: &mut crate::in_play::Database,
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
                duration: EffectDuration::Permanently,
                reason: None,
            });
            results.push_settled(ActionResult::ManifestTopOfLibrary(
                target.id().unwrap().controller(db),
            ));
        }
    }
}
