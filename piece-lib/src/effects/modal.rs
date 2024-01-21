use crate::{
    effects::{EffectBehaviors, Mode},
    pending_results::Source,
    protogen::effects::{effect::Effect, Modes},
};

impl EffectBehaviors for Modes {
    fn modes(&self) -> Vec<Mode> {
        self.modes.clone()
    }

    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: &crate::protogen::ids::CardId,
        controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if let Some(mode) = results.chosen_modes().pop() {
            for effect in self.modes[mode].effects.iter() {
                effect
                    .effect
                    .as_ref()
                    .unwrap()
                    .push_pending_behavior(db, source, controller, results);
            }
        } else {
            results.push_choose_mode(Source::Effect(Effect::from(self.clone()), source.clone()));
        }
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        source: &crate::protogen::ids::CardId,
        controller: &crate::protogen::ids::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
