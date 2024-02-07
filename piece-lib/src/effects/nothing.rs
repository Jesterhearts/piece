use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::Nothing,
};

impl EffectBehaviors for Nothing {
    fn apply(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        vec![]
    }
}
