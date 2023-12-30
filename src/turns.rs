use std::collections::HashSet;

use bevy_ecs::{event::Events, system::Resource};
use indexmap::IndexSet;

use crate::{
    battlefield::{Battlefield, PendingResults},
    controller::ControllerRestriction,
    in_play::{
        set_current_turn, AttackingBannedThisTurn, CardId, Database, DeleteAbility, LifeGained,
        TimesDescended, TriggerId,
    },
    player::{AllPlayers, Owner},
    stack::Stack,
    triggers::trigger_source,
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

#[derive(Debug, Resource)]
pub struct Turn {
    pub turn_count: usize,
    pub phase: Phase,
    turn_order: Vec<Owner>,
    active_player: usize,
    priority_player: usize,
    passed: usize,
}

impl Turn {
    pub fn new(all_players: &AllPlayers) -> Self {
        let turn_order = all_players.all_players();

        Self {
            turn_count: 0,
            phase: Phase::default(),
            turn_order,
            active_player: 0,
            priority_player: 0,
            passed: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
    }

    pub fn step_priority(&mut self) {
        self.priority_player = (self.priority_player + 1) % self.turn_order.len();
        self.passed = 0;
    }

    pub fn pass_priority(&mut self) {
        self.priority_player = (self.priority_player + 1) % self.turn_order.len();
        self.passed = (self.passed + 1) % self.turn_order.len();
    }

    pub fn step(&mut self, db: &mut Database, all_players: &mut AllPlayers) -> PendingResults {
        if self.passed != 0 {
            return PendingResults::default();
        }

        self.priority_player = self.active_player;
        if !Stack::is_empty(db) {
            return Stack::resolve_1(db);
        }

        match self.phase {
            Phase::Untap => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::Upkeep;
                PendingResults::default()
            }
            Phase::Upkeep => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::Draw;
                if self.turn_count != 0 {
                    return all_players[self.active_player()].draw(db, 1);
                }
                PendingResults::default()
            }
            Phase::Draw => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::PreCombatMainPhase;

                let mut results = PendingResults::default();

                for trigger in
                    TriggerId::active_triggers_of_source::<trigger_source::PreCombatMainPhase>(db)
                {
                    match trigger.controller_restriction(db) {
                        ControllerRestriction::Any => {}
                        ControllerRestriction::You => {
                            if trigger.listener(db).controller(db) != self.active_player() {
                                continue;
                            }
                        }
                        ControllerRestriction::Opponent => {
                            if trigger.listener(db).controller(db) == self.active_player() {
                                continue;
                            }
                        }
                    }

                    results.extend(Stack::move_trigger_to_stack(db, trigger));
                }

                results
            }
            Phase::PreCombatMainPhase => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::BeginCombat;
                let mut results = PendingResults::default();
                for trigger in
                    TriggerId::active_triggers_of_source::<trigger_source::StartOfCombat>(db)
                {
                    match trigger.controller_restriction(db) {
                        ControllerRestriction::Any => {}
                        ControllerRestriction::You => {
                            if trigger.listener(db).controller(db) != self.active_player() {
                                continue;
                            }
                        }
                        ControllerRestriction::Opponent => {
                            if trigger.listener(db).controller(db) == self.active_player() {
                                continue;
                            }
                        }
                    }

                    results.extend(Stack::move_trigger_to_stack(db, trigger));
                }
                results
            }
            Phase::BeginCombat => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::DeclareAttackers;
                let mut results = PendingResults::default();
                results.set_declare_attackers(db, all_players, self.active_player());
                results
            }
            Phase::DeclareAttackers => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::DeclareBlockers;
                PendingResults::default()
            }
            Phase::DeclareBlockers => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::FirstStrike;
                PendingResults::default()
            }
            Phase::FirstStrike => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::Damage;
                let cards = CardId::all_attackers(db);
                // TODO blocks
                for (card, target) in cards {
                    if let Some(power) = card.power(db) {
                        if power > 0 {
                            all_players[target].life_total -= power;
                        }
                    }
                }
                PendingResults::default()
            }
            Phase::Damage => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                CardId::clear_all_attacking(db);
                self.phase = Phase::PostCombatMainPhase;
                PendingResults::default()
            }
            Phase::PostCombatMainPhase => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::EndStep;

                let mut results = PendingResults::default();

                for trigger in TriggerId::active_triggers_of_source::<trigger_source::EndStep>(db) {
                    match trigger.controller_restriction(db) {
                        ControllerRestriction::Any => {}
                        ControllerRestriction::You => {
                            if trigger.listener(db).controller(db) != self.active_player() {
                                continue;
                            }
                        }
                        ControllerRestriction::Opponent => {
                            if trigger.listener(db).controller(db) == self.active_player() {
                                continue;
                            }
                        }
                    }

                    if !trigger.listener(db).passes_restrictions(
                        db,
                        trigger.listener(db),
                        trigger.controller_restriction(db),
                        &trigger.restrictions(db),
                    ) {
                        continue;
                    }

                    results.extend(Stack::move_trigger_to_stack(db, trigger));
                }

                results
            }
            Phase::EndStep => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                self.phase = Phase::Cleanup;

                Battlefield::end_turn(db)
            }
            Phase::Cleanup => {
                for player in all_players.all_players() {
                    all_players[player].mana_pool.drain();
                }
                CardId::cleanup_tokens_in_limbo(db);
                TriggerId::cleanup_temporary_triggers(db);

                db.remove_resource::<LifeGained>();
                db.remove_resource::<TimesDescended>();
                db.remove_resource::<AttackingBannedThisTurn>();

                let mut events = db.resource_mut::<Events<DeleteAbility>>();
                let events = events.drain().collect::<HashSet<_>>();
                for event in events {
                    event.ability.delete(db);
                }

                self.phase = Phase::Untap;
                self.active_player = (self.active_player + 1) % self.turn_order.len();
                self.priority_player = self.active_player;

                set_current_turn(db, self.active_player(), self.turn_count);

                Battlefield::untap(db, self.active_player());
                self.turn_count += 1;
                PendingResults::default()
            }
        }
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

    pub fn passed_full_round(&self) -> bool {
        self.passed == 0
    }

    pub fn priority_player(&self) -> Owner {
        self.turn_order[self.priority_player]
    }
}
