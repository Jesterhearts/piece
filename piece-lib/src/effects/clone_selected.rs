use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::CloneSelected,
};

impl EffectBehaviors for CloneSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        if selected.len() > 1 {
            let cloning = selected.first().unwrap();
            let cloned = selected.last().unwrap();
            cloning
                .id(db)
                .unwrap()
                .clone_card(db, cloned.id(db).unwrap());
        }
    }
}
