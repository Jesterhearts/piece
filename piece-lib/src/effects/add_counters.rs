use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::AddCounters,
};

impl EffectBehaviors for AddCounters {
    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        for target in selected.iter() {
            if let Some(id) = target.id(db) {
                *db[id]
                    .counters
                    .entry(self.counter.enum_value().unwrap())
                    .or_default() += self.count.count(db, source, selected) as u32;
            } else {
                todo!("Handle counters on players");
            }
        }
    }
}
