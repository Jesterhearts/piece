use crate::{
    effects::{
        move_to_graveyard::move_card_to_graveyard, EffectBehaviors, EffectBundle, SelectedStack,
    },
    in_play::{CardId, Database},
    protogen::effects::Sacrifice,
};

impl EffectBehaviors for Sacrifice {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        move_card_to_graveyard(db, selected, source)
    }
}
