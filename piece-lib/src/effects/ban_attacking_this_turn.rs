use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::BanAttackingThisTurn,
};

impl EffectBehaviors for BanAttackingThisTurn {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        db.all_players[selected.last().unwrap().player().unwrap()].ban_attacking_this_turn = true;
        vec![]
    }
}
