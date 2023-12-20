use indexmap::IndexSet;

use crate::{
    battlefield::{Battlefield, PendingResults},
    in_play::{CardId, Database},
    player::{AllPlayers, Owner},
    stack::Stack,
    types::Type,
};

#[derive(Debug, Default, strum::AsRefStr)]
pub enum Phase {
    #[default]
    Untap,
    Upkeep,
    Draw,
    PreCombatMainPhase,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    FirstStrike,
    Damage,
    PostCombatMainPhase,
    EndStep,
    Cleanup,
}

#[derive(Debug)]
pub struct Turn {
    pub turn_count: usize,
    pub phase: Phase,
    turn_order: Vec<Owner>,
    active_player: usize,
}

impl Turn {
    pub fn new(players: &AllPlayers) -> Self {
        let turn_order = players.all_players();

        Self {
            turn_count: 0,
            phase: Phase::default(),
            turn_order,
            active_player: 0,
        }
    }

    #[cfg(test)]
    pub fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
    }

    pub fn step(&mut self, db: &mut Database, all_players: &mut AllPlayers) -> PendingResults {
        match self.phase {
            Phase::Untap => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::Upkeep;
            }
            Phase::Upkeep => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::Draw;
                if self.turn_count != 0 {
                    return all_players[self.active_player()].draw(db, 1);
                }
            }
            Phase::Draw => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::PreCombatMainPhase;
            }
            Phase::PreCombatMainPhase => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::BeginCombat;
            }
            Phase::BeginCombat => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::DeclareAttackers;
            }
            Phase::DeclareAttackers => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::DeclareBlockers;
            }
            Phase::DeclareBlockers => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::FirstStrike;
            }
            Phase::FirstStrike => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::Damage;
            }
            Phase::Damage => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::PostCombatMainPhase;
            }
            Phase::PostCombatMainPhase => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::EndStep;
            }
            Phase::EndStep => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::Cleanup;

                Battlefield::end_turn(db);
                let results = Battlefield::check_sba(db);

                return Battlefield::apply_action_results(db, all_players, &results);
            }
            Phase::Cleanup => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::Untap;
                self.active_player = (self.active_player + 1) % self.turn_order.len();
                Battlefield::untap(db, self.active_player());
                self.turn_count += 1;
            }
        }

        PendingResults::default()
    }

    pub fn can_cast(&self, db: &mut Database, card: CardId) -> bool {
        let instant_or_flash =
            card.types_intersect(db, &IndexSet::from([Type::Instant])) || card.has_flash(db);
        // TODO teferi like effects.
        if instant_or_flash {
            return true;
        }

        let active_player = self.active_player();
        if card.controller(db) == active_player
            && matches!(
                self.phase,
                Phase::PreCombatMainPhase | Phase::PostCombatMainPhase
            )
            && Stack::is_empty(db)
        {
            return true;
        }

        false
    }

    pub fn active_player(&self) -> Owner {
        self.turn_order[self.active_player]
    }
}
