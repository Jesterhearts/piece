use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::Log,
    protogen::effects::SelectSource,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectSource {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        Log::card_chosen(db, source.unwrap());
        selected.push(Selected {
            location: source.unwrap().location(db),
            target_type: TargetType::Card(source.unwrap()),
            targeted: false,
            restrictions: vec![],
        });

        vec![]
    }
}
