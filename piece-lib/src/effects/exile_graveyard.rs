use itertools::Itertools;

use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{Duration, ExileGraveyard},
};

impl EffectBehaviors for ExileGraveyard {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        for target in selected.iter().map(|target| target.player().unwrap()) {
            for card in db
                .owner_view_mut(target)
                .graveyard
                .iter()
                .copied()
                .collect_vec()
            {
                card.move_to_exile(db, source.unwrap(), None, Duration::PERMANENTLY);
            }
        }

        vec![]
    }
}
