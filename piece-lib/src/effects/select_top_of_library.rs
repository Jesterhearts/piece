use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    log::Log,
    protogen::{effects::SelectTopOfLibrary, targets::Location},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectTopOfLibrary {
    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let targets = selected.clone();
        selected.clear();

        let count = self.count.count(db, source, &targets);
        for target in targets.iter() {
            let player = target.player().unwrap();
            for card in db.all_players[player]
                .library
                .cards
                .iter()
                .copied()
                .rev()
                .take(count as usize)
                .collect_vec()
            {
                Log::card_chosen(db, card);
                selected.push(Selected {
                    location: Some(Location::IN_LIBRARY),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                })
            }
        }
    }
}
