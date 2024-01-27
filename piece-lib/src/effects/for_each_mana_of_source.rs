use crate::{
    effects::{EffectBehaviors, EffectBundle, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::ForEachManaOfSource,
};

impl EffectBehaviors for ForEachManaOfSource {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let source = source.unwrap();
        if let Some(from_source) = db[source]
            .sourced_mana
            .get(&self.source.enum_value().unwrap())
            .copied()
        {
            for _ in 0..from_source {
                pending.push_back(EffectBundle {
                    selected: selected.clone(),
                    effects: self.effects.to_vec(),
                    source: Some(source),
                    ..Default::default()
                });
            }
        }
    }
}
