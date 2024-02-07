use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::{Log, LogId},
    protogen::effects::SelectAll,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectAll {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        for card in db.cards.keys().copied().collect_vec() {
            if card.passes_restrictions(db, LogId::current(db), source.unwrap(), &self.restrictions)
            {
                Log::card_chosen(db, card);
                selected.push(Selected {
                    location: card.location(db),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: self.restrictions.clone(),
                });
            }
        }

        vec![]
    }
}
