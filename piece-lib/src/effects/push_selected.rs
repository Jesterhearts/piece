use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::PushSelected,
};

impl EffectBehaviors for PushSelected {
    fn apply(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        selected.save();

        vec![]
    }
}
