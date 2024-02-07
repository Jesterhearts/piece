use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::GainLife,
};

impl EffectBehaviors for GainLife {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let target = selected.first().unwrap().player().unwrap();
        let count = self.count.count(db, source, selected);
        db.all_players[target].life_total += count;
        db.all_players[target].life_gained_this_turn += count as u32;

        vec![]
    }
}
