use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{Effect, MultiplyTokens},
};

impl EffectBehaviors for MultiplyTokens {
    fn apply(
        &mut self,
        _db: &mut Database,
        _pending: &mut PendingEffects,
        _source: Option<CardId>,
        _selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        unreachable!()
    }

    fn apply_replacement(&self, effect: Effect) -> Vec<Effect> {
        let mut replaced = vec![];
        for _ in 0..self.multiplier {
            replaced.push(effect.clone());
        }

        replaced
    }
}
