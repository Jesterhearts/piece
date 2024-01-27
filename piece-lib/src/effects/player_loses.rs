use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::PlayerLoses,
};

impl EffectBehaviors for PlayerLoses {
    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let target = selected.first().unwrap().player().unwrap();
        db.all_players[target].lost = true;
    }
}
