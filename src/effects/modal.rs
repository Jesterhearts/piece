use crate::{
    battlefield::Source,
    effects::{Effect, EffectBehaviors, Mode},
    protogen,
};

#[derive(Debug)]
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
    fn modes(&'static self) -> Vec<Mode> {
        self.modes.clone()
    }

    fn needs_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &'static self,
        _db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if let Some(mode) = results.chosen_modes().pop() {
            for effect in self.modes[mode].effects.iter() {
                effect
                    .effect
                    .push_pending_behavior(db, source, controller, results);
            }
        } else {
            results.push_choose_mode(Source::Effect(Effect(self), source));
        }
    }

    fn push_behavior_with_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
