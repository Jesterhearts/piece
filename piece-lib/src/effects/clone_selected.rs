use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::CloneSelected,
};

impl EffectBehaviors for CloneSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        if selected.len() > 1 {
            let cloning = selected.first().unwrap();
            let cloned = selected.last().unwrap();
            cloning
                .id(db)
                .unwrap()
                .clone_card(db, cloned.id(db).unwrap());
        }

        vec![]
    }
}
