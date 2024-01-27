use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    log::Log,
    protogen::effects::SelectSelf,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectSelf {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
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
