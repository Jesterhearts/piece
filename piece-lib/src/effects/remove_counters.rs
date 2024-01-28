use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::RemoveCounters,
};

impl EffectBehaviors for RemoveCounters {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let count = self.count.count(db, source, selected);
        let target = selected.first().unwrap().id(db).unwrap();
        *db[target]
            .counters
            .entry(self.counter.enum_value().unwrap())
            .or_default() = db[target]
            .counters
            .entry(self.counter.enum_value().unwrap())
            .or_default()
            .saturating_sub(count as u32);

        vec![]
    }
}
