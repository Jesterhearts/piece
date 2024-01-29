use crate::{
    effects::{ApplyResult, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{CounterSpell, MoveToGraveyard},
};

impl EffectBehaviors for CounterSpell {
    fn apply(
        &mut self,
        _db: &mut Database,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        vec![ApplyResult::PushBack(EffectBundle {
            source,
            effects: vec![MoveToGraveyard::default().into()],
            ..Default::default()
        })]
    }
}
