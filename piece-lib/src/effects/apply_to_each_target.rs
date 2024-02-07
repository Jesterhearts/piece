use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{ApplyToEachTarget, PopSelected},
};

impl EffectBehaviors for ApplyToEachTarget {
    fn apply(
        &mut self,
        _db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let mut results = vec![];
        for target in selected.current.clone().into_iter().rev() {
            let mut effects = self.effects.clone();
            effects.push(PopSelected::default().into());
            results.push(EffectBundle {
                push_on_enter: Some(vec![target]),
                effects,
                source,
                ..Default::default()
            });
        }

        results
    }
}
