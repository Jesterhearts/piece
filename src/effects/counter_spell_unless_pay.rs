use anyhow::anyhow;
use itertools::Itertools;
use tracing::Level;

use crate::{
    effects::{counter_spell::CounterSpellOrAbility, Effect, EffectBehaviors},
    mana::ManaCost,
    pending_results::{
        choose_targets::ChooseTargets,
        pay_costs::{self, PayCost, SpendMana},
        TargetSource,
    },
    player::mana_pool::SpendReason,
    protogen,
    stack::{ActiveTarget, Entry, StackEntry},
};

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
        _controller: crate::player::Controller,
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for id in db
            .stack
            .entries
            .iter()
            .enumerate()
            .filter_map(|(id, entry)| {
                if let StackEntry {
                    ty: Entry::Card(card),
                    ..
                } = entry
                {
                    if card.can_be_countered(db, source, &[]) {
                        Some(id)
                    } else {
                        None
                    }
                } else if let StackEntry {
                    ty: Entry::Ability { .. },
                    ..
                } = entry
                {
                    Some(id)
                } else {
                    None
                }
            })
        {
            targets.push(ActiveTarget::Stack { id });
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
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            source,
        ));
    }

    #[instrument(level = Level::INFO, skip(db, results))]
    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if let Ok(ActiveTarget::Stack { id }) = targets.iter().exactly_one() {
            match self.cost {
                Cost::Fixed(count) => {
                    results.push_pay_costs(PayCost::new_or_else(
                        db.stack.entries[*id].ty.source(),
                        pay_costs::Cost::SpendMana(SpendMana::new(
                            vec![ManaCost::Generic(count)],
                            SpendReason::Other,
                        )),
                        vec![Effect::CounterSpellOrAbility(CounterSpellOrAbility {
                            restrictions: Default::default(),
                        })],
                        targets,
                    ));
                }
            }
        } else {
            warn!("Skipping targeting");
        }
    }
}
