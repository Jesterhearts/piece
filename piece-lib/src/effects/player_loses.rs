use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::PlayerLoses,
};

impl EffectBehaviors for PlayerLoses {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let target = selected.first().unwrap().player().unwrap();
        db.all_players[target].lost = true;

        vec![]
    }
}
