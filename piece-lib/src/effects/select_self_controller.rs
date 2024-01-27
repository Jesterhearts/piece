use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::SelectSelfController,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectSelfController {
    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        selected.push(Selected {
            location: None,
            target_type: TargetType::Player(db[source.unwrap()].controller.into()),
            targeted: true,
            restrictions: vec![],
        })
    }
}
