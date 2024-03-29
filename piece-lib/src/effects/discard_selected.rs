use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::Log,
    protogen::effects::{DiscardSelected, MoveToGraveyard},
};

impl EffectBehaviors for DiscardSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        for target in selected
            .iter()
            .map(|target| target.id(db).unwrap())
            .collect_vec()
        {
            Log::discarded(db, target)
        }

        vec![EffectBundle {
            source,
            effects: vec![MoveToGraveyard::default().into()],
            ..Default::default()
        }]
    }
}
