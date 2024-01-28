use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::Transform,
};

impl EffectBehaviors for Transform {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        for target in selected.iter() {
            target.id(db).unwrap().transform(db);
        }

        vec![]
    }
}
