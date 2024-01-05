use crate::{
    effects::{Effect, EffectBehaviors, Mode},
    pending_results::Source,
    protogen,
};

#[derive(Debug, Clone)]
pub(crate) struct Modal {
    modes: Vec<Mode>,
}

impl TryFrom<&protogen::effects::Modes> for Modal {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Modes) -> Result<Self, Self::Error> {
        Ok(Self {
            modes: value
                .modes
                .iter()
                .map(Mode::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for Modal {
    fn modes(&self) -> Vec<Mode> {
        self.modes.clone()
    }

    fn needs_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if let Some(mode) = results.chosen_modes().pop() {
            for effect in self.modes[mode].effects.iter() {
                effect
                    .effect
                    .push_pending_behavior(db, source, controller, results);
            }
        } else {
            results.push_choose_mode(Source::Effect(Effect::from(self.clone()), source));
        }
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
