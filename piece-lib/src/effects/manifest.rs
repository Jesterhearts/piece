use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    player::Player,
    protogen::effects::Manifest,
};

impl EffectBehaviors for Manifest {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let target = selected.first().unwrap().player().unwrap();
        pending.extend(Player::manifest(db, target));
    }
}
