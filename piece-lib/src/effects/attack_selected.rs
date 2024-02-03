use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::AttackSelected,
};

impl EffectBehaviors for AttackSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let attacker = selected.last().unwrap();
        let target = selected.first().unwrap();
        let attacker = attacker.id(db).unwrap();
        db[attacker].attacking = target.player();

        vec![]
    }
}
