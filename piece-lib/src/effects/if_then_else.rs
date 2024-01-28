use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::IfThenElse,
};

impl EffectBehaviors for IfThenElse {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        modes: &[usize],
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut results = vec![];
        if selected
            .iter()
            .flat_map(|selected| selected.id(db))
            .all(|card| {
                card.passes_restrictions(db, LogId::current(db), source.unwrap(), &self.if_)
            })
        {
            for effect in self.then.iter_mut() {
                results.extend(effect.effect.as_mut().unwrap().apply(
                    db,
                    source,
                    selected,
                    modes,
                    skip_replacement,
                ));
            }
        } else {
            for effect in self.else_.iter_mut() {
                results.extend(effect.effect.as_mut().unwrap().apply(
                    db,
                    source,
                    selected,
                    modes,
                    skip_replacement,
                ));
            }
        }

        results
    }
}
