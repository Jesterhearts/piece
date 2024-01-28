use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::SelectSelfController,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectSelfController {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        selected.push(Selected {
            location: None,
            target_type: TargetType::Player(db[source.unwrap()].controller.into()),
            targeted: true,
            restrictions: vec![],
        });

        vec![]
    }
}
