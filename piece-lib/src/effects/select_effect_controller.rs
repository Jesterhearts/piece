use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::SelectEffectController,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectEffectController {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let new_selection = Selected {
            location: None,
            target_type: TargetType::Player(self.priority(db, source, selected, &selected.modes)),
            targeted: true,
            restrictions: vec![],
        };
        selected.push(new_selection);

        vec![]
    }
}
