use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::AttackSelected,
};

impl EffectBehaviors for AttackSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let attacker = selected.last().unwrap();
        let target = selected.first().unwrap();
        let attacker = attacker.id(db).unwrap();
        db[attacker].attacking = target.player();
    }
}
