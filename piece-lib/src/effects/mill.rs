use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::Mill,
};

impl EffectBehaviors for Mill {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let target = selected.first().unwrap().player().unwrap();
        let count = self.count.count(db, source, selected);
        for _ in 0..count {
            if let Some(card) = db.all_players[target].library.draw() {
                card.move_to_graveyard(db);
            }
        }

        vec![]
    }
}
