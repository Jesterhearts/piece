use rand::{seq::SliceRandom, thread_rng};

use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::ShuffleSelected,
};

impl EffectBehaviors for ShuffleSelected {
    fn apply(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        selected.shuffle(&mut thread_rng());

        vec![]
    }
}
