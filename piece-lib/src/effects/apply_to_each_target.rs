use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::ApplyToEachTarget,
};

impl EffectBehaviors for ApplyToEachTarget {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut results = vec![];
        for selected in selected.iter() {
            for effect in self.effects.iter_mut() {
                results.extend(effect.effect.as_mut().unwrap().apply(
                    db,
                    source,
                    &mut SelectedStack::new(vec![selected.clone()]),
                    skip_replacement,
                ));
            }
        }

        results
    }
}
