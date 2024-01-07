use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors},
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen,
    stack::{ActiveTarget, Entry, StackEntry},
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
        for (stack_id, card) in db
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
                    Some((id, card))
                } else {
                    None
                }
            })
        {
            if card.can_be_countered(db, source, &self.restrictions) {
                targets.push(ActiveTarget::Stack { id: stack_id });
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
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
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
        results: &mut crate::pending_results::PendingResults,
    ) {
        for target in targets {
            let ActiveTarget::Stack { id } = target else {
                unreachable!()
            };

            results.push_settled(ActionResult::SpellCountered {
                id: db.stack.entries[id].ty.clone(),
            });
        }
    }
}
