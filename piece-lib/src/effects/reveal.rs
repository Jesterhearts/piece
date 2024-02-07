use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::Reveal,
};

impl EffectBehaviors for Reveal {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        for target in selected.iter() {
            let target = target.id(db).unwrap();
            db[target].revealed = true;
        }

        vec![]
    }
}
