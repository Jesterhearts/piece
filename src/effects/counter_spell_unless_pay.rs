use anyhow::anyhow;
use bevy_ecs::entity::Entity;
use itertools::Itertools;
use tracing::Level;

use crate::{
    battlefield::{
        choose_targets::ChooseTargets,
        pay_costs::{PayCost, SpendMana},
        TargetSource,
    },
    effects::{Effect, EffectBehaviors},
    in_play::{CardId, InStack},
    mana::ManaCost,
    player::mana_pool::SpendReason,
    protogen,
    stack::{ActiveTarget, Stack},
};

#[derive(Debug)]
pub(crate) enum Cost {
    Fixed(usize),
}

impl TryFrom<&protogen::effects::counter_spell_unless_pay::Cost> for Cost {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::counter_spell_unless_pay::Cost,
    ) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::counter_spell_unless_pay::Cost::Fixed(value) => {
                Ok(Self::Fixed(usize::try_from(value.count)?))
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct CounterSpellUnlessPay {
    cost: Cost,
}

impl TryFrom<&protogen::effects::CounterSpellUnlessPay> for CounterSpellUnlessPay {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::CounterSpellUnlessPay) -> Result<Self, Self::Error> {
        Ok(Self {
            cost: value
                .cost
                .as_ref()
                .ok_or_else(|| anyhow!("Expected cost to have a cost specified"))
                .and_then(Cost::try_from)?,
        })
    }
}

impl EffectBehaviors for CounterSpellUnlessPay {
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
        &'static self,
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
            if card.can_be_countered(db, source, &[]) {
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

    #[instrument(level = Level::INFO, skip(db, results))]
    fn push_behavior_with_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if let Ok(ActiveTarget::Stack { id }) = targets.into_iter().exactly_one() {
            match self.cost {
                Cost::Fixed(count) => {
                    results.push_pay_costs(PayCost::SpendMana(SpendMana::new(
                        vec![ManaCost::Generic(count)],
                        Stack::in_stack(db).get(&id).unwrap().source(),
                        SpendReason::Other,
                    )));
                }
            }
        } else {
            warn!("Skipping targetting");
        }
    }
}
