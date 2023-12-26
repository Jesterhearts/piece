use crate::{
    battlefield::{PendingResult, Source},
    in_play::Database,
    player::AllPlayers,
};

#[derive(Debug)]
pub struct ChooseModes {
    pub source: Source,
}

impl PendingResult for ChooseModes {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        false
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
            if let Source::Effect(effect, source) = &self.source {
                effect.push_pending_behavior(db, *source, source.controller(db), results);
            }
            true
        } else {
            false
        }
    }
}
