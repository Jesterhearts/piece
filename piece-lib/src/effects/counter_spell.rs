use crate::{
    effects::{ApplyResult, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::{CounterSpell, MoveToGraveyard},
};

impl EffectBehaviors for CounterSpell {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        selected.current.retain(|target| {
            target
                .id(db)
                .unwrap()
                .can_be_countered(db, LogId::current(db), source.unwrap(), &[])
        });

        vec![ApplyResult::PushBack(EffectBundle {
            source,
            effects: vec![MoveToGraveyard::default().into()],
            ..Default::default()
        })]
    }
}
