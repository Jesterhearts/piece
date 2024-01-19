use itertools::Itertools;

use crate::{
    effects::EffectBehaviors,
    in_play::Database,
    pending_results::{Options, PendingResult, Source},
    stack::ActiveTarget,
};

#[derive(Debug)]
pub(crate) struct ChooseModes {
    pub(crate) source: Source,
}

impl PendingResult for ChooseModes {
    fn cancelable(&self, _db: &Database) -> bool {
        true
    }

    fn options(&self, db: &mut Database) -> Options {
        Options::MandatoryList(self.source.mode_options(db))
    }

    fn target_for_option(&self, db: &Database, _option: usize) -> Option<ActiveTarget> {
        self.source.card().target_from_location(db)
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
        choice: Option<usize>,
        results: &mut super::PendingResults,
    ) -> bool {
        if let Some(choice) = choice {
            results.push_chosen_mode(choice);
            match &self.source {
                Source::Card(card) => {
                    for effect in card.faceup_face(db).modes[choice]
                        .effects
                        .iter()
                        .filter(|effect| {
                            effect.effect.as_ref().unwrap().wants_targets(db, *card) > 0
                        })
                        .cloned()
                        .collect_vec()
                    {
                        effect.effect.as_ref().unwrap().push_pending_behavior(
                            db,
                            *card,
                            db[*card].controller,
                            results,
                        );
                    }
                }
                Source::Effect(effect, source) => {
                    effect.push_pending_behavior(db, *source, db[*source].controller, results);
                }
                _ => {}
            }
            true
        } else {
            false
        }
    }
}
