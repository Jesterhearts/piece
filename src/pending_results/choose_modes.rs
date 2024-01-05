use itertools::Itertools;

use crate::{
    effects::EffectBehaviors,
    in_play::Database,
    pending_results::{PendingResult, Source},
    player::AllPlayers,
};

#[derive(Debug)]
pub(crate) struct ChooseModes {
    pub(crate) source: Source,
}

impl PendingResult for ChooseModes {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        true
    }

    fn options(&self, db: &mut Database, _all_players: &AllPlayers) -> Vec<(usize, String)> {
        self.source.mode_options(db)
    }

    fn description(&self, _db: &crate::in_play::Database) -> String {
        "mode".to_string()
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn make_choice(
        &mut self,
        db: &mut crate::in_play::Database,
        _all_players: &mut crate::player::AllPlayers,
        choice: Option<usize>,
        results: &mut super::PendingResults,
    ) -> bool {
        if let Some(choice) = choice {
            results.push_chosen_mode(choice);
            match &self.source {
                Source::Card(card) => {
                    for effect in card.modes(db).unwrap()[choice]
                        .effects
                        .iter()
                        .filter(|effect| effect.effect.wants_targets(db, *card) > 0)
                        .collect_vec()
                    {
                        effect.effect.push_pending_behavior(
                            db,
                            *card,
                            card.controller(db),
                            results,
                        );
                    }
                }
                Source::Effect(effect, source) => {
                    effect.push_pending_behavior(db, *source, source.controller(db), results);
                }
                _ => {}
            }
            true
        } else {
            false
        }
    }
}
