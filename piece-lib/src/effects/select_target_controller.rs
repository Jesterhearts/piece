use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::SelectTargetController,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectTargetController {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let target = selected.first().unwrap().id(db).unwrap();
        selected.clear();
        selected.push(Selected {
            location: None,
            target_type: TargetType::Player(db[target].controller.into()),
            targeted: false,
            restrictions: vec![],
        });

        vec![]
    }
}
