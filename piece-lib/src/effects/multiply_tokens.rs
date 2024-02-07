use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{Effect, MultiplyTokens},
};

impl EffectBehaviors for MultiplyTokens {
    fn apply(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
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
