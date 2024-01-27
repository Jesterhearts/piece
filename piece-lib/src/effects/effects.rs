use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::Effects,
};

impl EffectBehaviors for Effects {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        modes: &[usize],
        skip_replacement: bool,
    ) {
        for effect in self.effects.iter_mut() {
            effect.effect.as_mut().unwrap().apply(
                db,
                pending,
                source,
                selected,
                modes,
                skip_replacement,
            )
        }
    }
}
