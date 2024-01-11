use crate::{
    in_play::Database,
    pending_results::{PendingResults, ResolutionResult},
    player::Owner,
    turns::Turn,
};

pub struct AI {
    player: Owner,
}

impl AI {
    pub fn new(player: Owner) -> Self {
        Self { player }
    }

    pub fn priority(&self, db: &mut Database, pending: &mut PendingResults) -> PendingResults {
        while pending.priority(db) == self.player {
            let result = pending.resolve(db, Some(0));
            if result == ResolutionResult::Complete {
                break;
            }
        }

        db.turn.pass_priority();
        debug!(
            "Passing priority: full round {}",
            db.turn.passed_full_round()
        );

        if db.turn.passed_full_round() {
            let mut pending = Turn::step(db);
            debug!("Pending priority {:?}", pending.priority(db));
            if pending.priority(db) == self.player {
                self.priority(db, &mut pending)
            } else {
                pending
            }
        } else if db.turn.priority_player() == self.player {
            return self.priority(db, pending);
        } else {
            PendingResults::default()
        }
    }
}
