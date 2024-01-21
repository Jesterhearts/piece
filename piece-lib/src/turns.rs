use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    battlefield::Battlefields,
    in_play::Database,
    log::{Log, LogId},
    pending_results::PendingResults,
    player::{AllPlayers, Player},
    protogen::{
        ids::{ActivatedAbilityId, CardId, Owner},
        triggers::TriggerSource,
        types::Type,
    },
    stack::Stack,
    types::TypeSet,
};

#[derive(Debug, Default, PartialEq, Eq, strum::AsRefStr)]
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

#[derive(Debug, Default)]
pub struct Turn {
    pub turn_count: usize,
    pub phase: Phase,
    turn_order: Vec<Owner>,
    active_player: usize,
    priority_player: usize,
    passed: usize,

    pub(crate) number_of_attackers_this_turn: usize,
    pub(crate) activated_abilities: HashSet<ActivatedAbilityId>,
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

            number_of_attackers_this_turn: 0,
            activated_abilities: Default::default(),
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

    #[instrument(skip(db))]
    pub fn step(db: &mut Database) -> PendingResults {
        if db.turn.passed != 0 {
            return PendingResults::default();
        }

        db.turn.priority_player = db.turn.active_player;
        if !db.stack.is_empty() {
            return Stack::resolve_1(db);
        }

        match db.turn.phase {
            Phase::Untap => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::Upkeep;

                let mut results = PendingResults::default();
                let player = db.turn.active_player();

                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::UPKEEP) {
                    if !Owner::from(db[&listener].controller.clone()).passes_restrictions(
                        db,
                        LogId::current(db),
                        &player.clone().into(),
                        &trigger.trigger.restrictions,
                    ) {
                        continue;
                    }

                    results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                }

                results
            }
            Phase::Upkeep => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::Draw;
                if db.turn.turn_count != 0 {
                    let player = db.turn.active_player();
                    return Player::draw(db, &player, 1);
                }
                PendingResults::default()
            }
            Phase::Draw => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::PreCombatMainPhase;

                let mut results = PendingResults::default();
                let player = db.turn.active_player();

                for (listener, trigger) in
                    db.active_triggers_of_source(TriggerSource::PRE_COMBAT_MAIN_PHASE)
                {
                    if !Owner::from(db[&listener].controller.clone()).passes_restrictions(
                        db,
                        LogId::current(db),
                        &player.clone().into(),
                        &trigger.trigger.restrictions,
                    ) {
                        continue;
                    }

                    results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                }

                results
            }
            Phase::PreCombatMainPhase => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::BeginCombat;
                let mut results = PendingResults::default();
                let player = db.turn.active_player();
                for (listener, trigger) in
                    db.active_triggers_of_source(TriggerSource::START_OF_COMBAT)
                {
                    if !Owner::from(db[&listener].controller.clone()).passes_restrictions(
                        db,
                        LogId::current(db),
                        &player.clone().into(),
                        &trigger.trigger.restrictions,
                    ) {
                        continue;
                    }

                    results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                }
                results
            }
            Phase::BeginCombat => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::DeclareAttackers;
                let mut results = PendingResults::default();
                let player = db.turn.active_player();
                results.set_declare_attackers(db, &player);
                results
            }
            Phase::DeclareAttackers => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::DeclareBlockers;
                PendingResults::default()
            }
            Phase::DeclareBlockers => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::FirstStrike;

                let mut results = PendingResults::default();

                for (card, target) in db.battlefield[&db.turn.active_player()]
                    .iter()
                    .filter_map(|card| {
                        db[card]
                            .attacking
                            .as_ref()
                            .filter(|_| card.first_strike(db) || card.double_strike(db))
                            .map(|attacking| (card.clone(), attacking.clone()))
                    })
                    .collect_vec()
                {
                    if let Some(power) = card.power(db) {
                        if power > 0 {
                            db.all_players[&target].life_total -= power;

                            for (listener, trigger) in db.active_triggers_of_source(
                                TriggerSource::DEALS_COMBAT_DAMAGE_TO_PLAYER,
                            ) {
                                if card.passes_restrictions(
                                    db,
                                    LogId::current(db),
                                    &listener,
                                    &trigger.trigger.restrictions,
                                ) {
                                    results.extend(Stack::move_trigger_to_stack(
                                        db, listener, trigger,
                                    ));
                                }
                            }
                        }
                    }
                }

                results
            }
            Phase::FirstStrike => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::Damage;

                let mut results = PendingResults::default();

                // TODO blocks
                for (card, target) in db.battlefield[&db.turn.active_player()]
                    .iter()
                    .filter_map(|card| {
                        db[card]
                            .attacking
                            .as_ref()
                            .filter(|_| !card.first_strike(db))
                            .map(|attacking| (card.clone(), attacking.clone()))
                    })
                    .collect_vec()
                {
                    if let Some(power) = card.power(db) {
                        if power > 0 {
                            db.all_players[&target].life_total -= power;

                            for (listener, trigger) in db.active_triggers_of_source(
                                TriggerSource::DEALS_COMBAT_DAMAGE_TO_PLAYER,
                            ) {
                                if card.passes_restrictions(
                                    db,
                                    LogId::current(db),
                                    &listener,
                                    &trigger.trigger.restrictions,
                                ) {
                                    results.extend(Stack::move_trigger_to_stack(
                                        db, listener, trigger,
                                    ));
                                }
                            }
                        }
                    }
                }

                results
            }
            Phase::Damage => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }

                for card in db.battlefield[&db.turn.active_player()].iter() {
                    db.cards.get_mut(card).unwrap().attacking = None;
                }

                db.turn.phase = Phase::PostCombatMainPhase;
                PendingResults::default()
            }
            Phase::PostCombatMainPhase => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::EndStep;

                let mut results = PendingResults::default();
                let player = db.turn.active_player();

                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::END_STEP) {
                    if !Owner::from(db[&listener].controller.clone()).passes_restrictions(
                        db,
                        LogId::current(db),
                        &player.clone().into(),
                        &trigger.trigger.restrictions,
                    ) || !listener.passes_restrictions(
                        db,
                        LogId::current(db),
                        &listener,
                        &trigger.trigger.restrictions,
                    ) {
                        continue;
                    }

                    results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                }

                results
            }
            Phase::EndStep => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }
                db.turn.phase = Phase::Cleanup;

                let player = db.turn.active_player();
                let mut pending = Battlefields::end_turn(db);
                let hand_size = db.all_players[&player].hand_size;
                let in_hand = &db.hand[&player];
                if in_hand.len() > hand_size {
                    let discard = in_hand.len() - hand_size;
                    pending
                        .push_choose_discard(in_hand.iter().cloned().collect_vec(), discard as u32);
                }
                pending
            }
            Phase::Cleanup => {
                for player in db.all_players.all_players() {
                    db.all_players[&player].mana_pool.drain();
                }

                CardId::cleanup_tokens_in_limbo(db);
                db.graveyard.descended_this_turn.clear();
                db.turn.number_of_attackers_this_turn = 0;

                for player in db.all_players.all_players() {
                    let player = &mut db.all_players[&player];
                    player.lands_played_this_turn = 0;
                    player.life_gained_this_turn = 0;
                    player.ban_attacking_this_turn = false;
                }

                for ability in db.gc_abilities.drain(..) {
                    db.activated_abilities.remove(&ability);
                }

                db.turn.phase = Phase::Untap;
                db.turn.active_player = (db.turn.active_player + 1) % db.turn.turn_order.len();
                db.turn.priority_player = db.turn.active_player;

                db.turn.turn_count += 1;

                Log::new_turn(db, db.turn.active_player().clone());

                Battlefields::untap(db, &db.turn.active_player());
                PendingResults::default()
            }
        }
    }

    pub fn can_cast(db: &Database, card: &CardId) -> bool {
        let instant_or_flash =
            card.types_intersect(db, &TypeSet::from([Type::INSTANT])) || card.has_flash(db);
        // TODO teferi like effects.
        if instant_or_flash && !db.stack.split_second(db) {
            return true;
        }

        let active_player = db.turn.active_player();
        if db[card].controller == active_player
            && matches!(
                db.turn.phase,
                Phase::PreCombatMainPhase | Phase::PostCombatMainPhase
            )
            && db.stack.is_empty()
        {
            return true;
        }

        false
    }

    pub fn active_player(&self) -> Owner {
        self.turn_order[self.active_player].clone()
    }

    pub fn passed_full_priority_round(&self) -> bool {
        self.passed == 0
    }

    pub fn turns_per_round(&self) -> usize {
        self.turn_order.len()
    }

    pub fn priority_player(&self) -> Owner {
        self.turn_order[self.priority_player].clone()
    }
}
