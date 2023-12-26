use crate::{
    battlefield::{PendingResults, ResolutionResult},
    in_play::Database,
    player::{AllPlayers, Owner},
    turns::Turn,
};

pub struct AI {
    player: Owner,
}

impl AI {
    pub fn new(player: Owner) -> Self {
        Self { player }
    }

    pub fn priority(
        &self,
        db: &mut Database,
        all_players: &mut AllPlayers,
        turn: &mut Turn,
        pending: &mut PendingResults,
    ) -> PendingResults {
        while pending.priority(db, all_players, turn) == self.player {
            let result = pending.resolve(db, all_players, turn, Some(0));
            if result == ResolutionResult::Complete {
                break;
            }
        }

        turn.pass_priority();
        assert_ne!(turn.priority_player(), self.player);
        if turn.passed_full_round() {
            let mut pending = turn.step(db, all_players);
            if pending.priority(db, all_players, turn) == self.player {
                self.priority(db, all_players, turn, &mut pending)
            } else {
                pending
            }
        } else {
            PendingResults::default()
        }
    }
}
