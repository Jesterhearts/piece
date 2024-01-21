use itertools::Itertools;

use piece_lib::{
    battlefield::Battlefields,
    in_play::Database,
    pending_results::{PendingResults, ResolutionResult},
    player::Player,
    protogen::ids::Owner,
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
        if pending.is_empty() && db.turn.active_player() == self.player {
            if matches!(db.turn.phase, Phase::PreCombatMainPhase)
                && Player::can_play_land(db, &self.player)
            {
                debug!("Playing land");
                if let Some(land) = db.hand[&self.player].iter().find(|card| card.is_land(db)) {
                    pending.extend(Player::play_card(db, &self.player, &land.clone()));
                } else {
                    debug!("Found no lands in hand");
                }
            } else if matches!(db.turn.phase, Phase::PostCombatMainPhase) {
                for land in db.battlefield[&self.player]
                    .iter()
                    .filter(|card| card.is_land(db))
                    .cloned()
                    .collect_vec()
                {
                    pending.extend(Battlefields::activate_ability(
                        db,
                        &None,
                        &self.player,
                        &land,
                        0,
                    ));
                }

                assert!(pending.only_immediate_results(db));

                let result = pending.resolve(db, None);
                assert_eq!(result, ResolutionResult::Complete);

                if let Some(card) = db.hand[&self.player].iter().find(|card| !card.is_land(db)) {
                    pending.extend(Player::play_card(db, &self.player, &card.clone()));
                }
            }
        }

        while pending.priority(db) == self.player {
            let result = if pending.options(db).is_empty() {
                let result = pending.resolve(db, None);
                if result == ResolutionResult::PendingChoice
                    && pending.options(db).is_empty()
                    && pending.can_cancel(db)
                {
                    debug!("Cancelling pending");
                    ResolutionResult::Complete
                } else {
                    result
                }
            } else {
                pending.resolve(db, Some(0))
            };

            if result == ResolutionResult::Complete {
                break;
            }
        }

        db.turn.pass_priority();
        debug!(
            "Passing priority: full round {}",
            db.turn.passed_full_priority_round()
        );

        if db.turn.passed_full_priority_round() {
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
