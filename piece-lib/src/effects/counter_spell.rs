use crate::{
    effects::{ApplyResult, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{CounterSpell, Effect, MoveToGraveyard},
};

impl EffectBehaviors for CounterSpell {
    fn apply(
        &mut self,
        _db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        selected.save();
        vec![ApplyResult::PushBack(EffectBundle {
            source,
            effects: vec![Effect {
                effect: Some(MoveToGraveyard::default().into()),
                ..Default::default()
            }],
            ..Default::default()
        })]
    }
}
