use bevy_ecs::entity::Entity;
use itertools::Itertools;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    effects::{Effect, EffectBehaviors},
    in_play::{CardId, InStack},
    protogen,
    stack::{ActiveTarget, Stack},
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct CounterSpell {
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::CounterSpell> for CounterSpell {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::CounterSpell) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for CounterSpell {
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
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let cards_in_stack = db
            .query::<(Entity, &InStack)>()
            .iter(db)
            .map(|(entity, in_stack)| (CardId::from(entity), *in_stack))
            .sorted_by_key(|(_, in_stack)| *in_stack)
            .collect_vec();

        let mut targets = vec![];
        for (card, stack_id) in cards_in_stack.into_iter() {
            if card.can_be_countered(db, source, &self.restrictions) {
                targets.push(ActiveTarget::Stack { id: stack_id });
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
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let in_stack = Stack::in_stack(db);
        for target in targets {
            let ActiveTarget::Stack { id } = target else {
                unreachable!()
            };

            results.push_settled(ActionResult::SpellCountered {
                id: *in_stack.get(&id).unwrap(),
            });
        }
    }
}
