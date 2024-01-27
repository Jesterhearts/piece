use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::LoseLife,
};

impl EffectBehaviors for LoseLife {
    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let target = selected.first().unwrap().player().unwrap();
        let count = self.count.count(db, source, selected);
        db.all_players[target].life_total -= count;
    }
}
