use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    effects::{Effect, EffectBehaviors},
    in_play::OnBattlefield,
    stack::ActiveTarget,
    types::Type,
};

#[derive(Debug)]
pub(crate) struct TargetCreatureExplores;

impl EffectBehaviors for TargetCreatureExplores {
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn valid_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        controller
            .get_cards_in::<OnBattlefield>(db)
            .into_iter()
            .filter(|card| {
                card.types_intersect(db, &IndexSet::from([Type::Creature]))
                    && card.can_be_targeted(db, controller)
            })
            .map(|card| ActiveTarget::Battlefield { id: card })
            .filter(|target| !already_chosen.contains(target))
            .collect_vec()
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
        &'static self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Explore {
            target: targets.into_iter().exactly_one().unwrap(),
        })
    }
}
