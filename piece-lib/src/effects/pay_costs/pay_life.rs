use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::pay_cost::PayLife,
};

impl EffectBehaviors for PayLife {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let controller = db[source.unwrap()].controller;
        let count = self.count.count(db, source, selected);
        db.all_players[controller].life_total -= count;

        vec![]
    }
}
