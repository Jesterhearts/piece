use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::Reveal,
};

impl EffectBehaviors for Reveal {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        for target in selected.iter() {
            let target = target.id(db).unwrap();
            db[target].revealed = true;
        }

        vec![]
    }
}
