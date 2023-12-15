use std::collections::HashSet;

use crate::{
    battlefield::{Battlefield, PendingResults},
    in_play::{CardId, Database},
    player::{AllPlayers, Owner},
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

    pub fn step(&mut self, db: &mut Database, all_players: &mut AllPlayers) -> PendingResults {
        match self.phase {
            Phase::Untap => {
                self.phase = Phase::Upkeep;
            }
            Phase::Upkeep => {
                self.phase = Phase::Draw;
                if self.turn_count != 0 {
                    return all_players[self.active_player()].draw(db, 1);
                }
            }
            Phase::Draw => {
                self.phase = Phase::PreCombatMainPhase;
            }
            Phase::PreCombatMainPhase => {
                self.phase = Phase::BeginCombat;
            }
            Phase::BeginCombat => {
                self.phase = Phase::DeclareAttackers;
            }
            Phase::DeclareAttackers => {
                self.phase = Phase::DeclareBlockers;
            }
            Phase::DeclareBlockers => {
                self.phase = Phase::FirstStrike;
            }
            Phase::FirstStrike => {
                self.phase = Phase::Damage;
            }
            Phase::Damage => {
                self.phase = Phase::PostCombatMainPhase;
            }
            Phase::PostCombatMainPhase => {
                self.phase = Phase::EndStep;
            }
            Phase::EndStep => {
                self.phase = Phase::Cleanup;

                Battlefield::end_turn(db);
                let results = Battlefield::check_sba(db);

                return Battlefield::apply_action_results(db, all_players, &results);
            }
            Phase::Cleanup => {
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
            card.types_intersect(db, &HashSet::from([Type::Instant])) || card.has_flash(db);
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
        {
            return true;
        }

        false
    }

    pub fn active_player(&self) -> Owner {
        self.turn_order[self.active_player]
    }
}
