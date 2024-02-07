use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    player::Player,
    protogen::effects::Manifest,
};

impl EffectBehaviors for Manifest {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let target = selected.first().unwrap().player().unwrap();
        Player::manifest(db, target).into_iter().collect_vec()
    }
}
