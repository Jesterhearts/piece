use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::PopSelected,
};

impl EffectBehaviors for PopSelected {
    fn apply(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let _ = selected.restore();

        vec![]
    }
}
