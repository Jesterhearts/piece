use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::SelectAllPlayers,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectAllPlayers {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        for player in db.all_players.all_players() {
            selected.push(Selected {
                location: None,
                target_type: TargetType::Player(player),
                targeted: false,
                restrictions: vec![],
            });
        }

        vec![]
    }
}
