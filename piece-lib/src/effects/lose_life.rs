use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::LoseLife,
};

impl EffectBehaviors for LoseLife {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let target = selected.first().unwrap().player().unwrap();
        let count = self.count.count(db, source, selected);
        db.all_players[target].life_total -= count;

        vec![]
    }
}
