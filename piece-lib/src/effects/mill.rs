use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::Mill,
};

impl EffectBehaviors for Mill {
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
        for _ in 0..count {
            if let Some(card) = db.all_players[target].library.top() {
                card.move_to_graveyard(db);
            }
        }
    }
}
