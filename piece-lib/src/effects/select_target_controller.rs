use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::SelectTargetController,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectTargetController {
    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let target = selected.first().unwrap().id(db).unwrap();
        selected.clear();
        selected.push(Selected {
            location: None,
            target_type: TargetType::Player(db[target].controller.into()),
            targeted: false,
            restrictions: vec![],
        });
    }
}
