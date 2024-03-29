use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::{counters::Counter, effects::Untap},
};

impl EffectBehaviors for Untap {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        for target in selected.iter() {
            let target = target.id(db).unwrap();

            let stun = db[target].counters.entry(Counter::STUN).or_default();
            if *stun > 0 {
                *stun -= 1;
            } else {
                target.untap(db);
            }
        }

        vec![]
    }
}
