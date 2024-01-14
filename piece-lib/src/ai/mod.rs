use crate::{
    in_play::Database,
    pending_results::{PendingResults, ResolutionResult},
    player::{Owner, Player},
    turns::{Phase, Turn},
};

pub struct AI {
    player: Owner,
}

impl AI {
    pub fn new(player: Owner) -> Self {
        Self { player }
    }

    pub fn priority(&self, db: &mut Database, pending: &mut PendingResults) -> PendingResults {
        if pending.is_empty()
            && db.turn.active_player() == self.player
            && matches!(db.turn.phase, Phase::PreCombatMainPhase)
            && Player::can_play_land(db, self.player)
        {
            debug!("Playing land");
            if let Some(land) = db.hand[self.player].iter().find(|card| card.is_land(db)) {
                pending.extend(Player::play_card(db, self.player, *land));
            } else {
                debug!("Found no lands in hand");
            }
        }

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
