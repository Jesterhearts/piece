use crate::{
    effects::{ApplyResult, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::ForEachManaOfSource,
};

impl EffectBehaviors for ForEachManaOfSource {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut pending = vec![];
        let source = source.unwrap();
        if let Some(from_source) = db[source]
            .sourced_mana
            .get(&self.source.enum_value().unwrap())
            .copied()
        {
            for _ in 0..from_source {
                pending.push(ApplyResult::PushBack(EffectBundle {
                    selected: selected.clone(),
                    effects: self.effects.to_vec(),
                    source: Some(source),
                    ..Default::default()
                }));
            }
        }

        pending
    }
}
