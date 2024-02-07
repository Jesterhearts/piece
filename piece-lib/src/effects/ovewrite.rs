use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{Effect, Overwrite},
};

impl EffectBehaviors for Overwrite {
    fn apply(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        unreachable!()
    }

    fn apply_replacement(&self, _effect: Effect) -> Vec<Effect> {
        self.effects.clone()
    }
}
