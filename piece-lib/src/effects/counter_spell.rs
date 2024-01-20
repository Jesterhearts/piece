use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    log::LogId,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::effects::{effect::Effect, CounterSpellOrAbility},
    stack::{ActiveTarget, Entry, StackEntry},
};

impl EffectBehaviors for CounterSpellOrAbility {
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
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for (stack_id, card) in db.stack.entries.iter().filter_map(|(id, entry)| {
            if let StackEntry {
                ty: Entry::Card(card),
                ..
            } = entry
            {
                Some((id, card))
            } else {
                None
            }
        }) {
            if card.can_be_countered(db, log_session, source, &self.restrictions) {
                targets.push(ActiveTarget::Stack { id: *stack_id });
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
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        for target in targets {
            if target.id(db).unwrap().can_be_countered(
                db,
                LogId::current(db),
                source,
                &self.restrictions,
            ) {
                let ActiveTarget::Stack { id } = target else {
                    unreachable!()
                };

                results.push_settled(ActionResult::SpellCountered { index: id });
            }
        }
    }
}
