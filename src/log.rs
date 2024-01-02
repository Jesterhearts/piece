use std::sync::atomic::{AtomicUsize, Ordering};

use bevy_ecs::system::Resource;
use indexmap::IndexMap;
use tracing::Level;

use crate::{
    effects::target_gains_counters::Counter,
    in_play::{current_turn, AbilityId, CardId, CounterId, Database, TriggerId},
    player::{Controller, Owner},
    targets::Restriction,
};

static NEXT_LOG_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogId(usize);

impl LogId {
    pub(crate) fn current() -> Self {
        Self(NEXT_LOG_ID.load(Ordering::Relaxed))
    }

    pub(crate) fn new() -> Self {
        Self(NEXT_LOG_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub enum LeaveReason {
    Exiled,
    PutIntoGraveyard,
    ReturnedToHand,
    ReturnedToLibrary,
}

#[derive(Debug)]
pub enum LogEntry {
    NewTurn {
        player: Owner,
    },
    LeftBattlefield {
        reason: LeaveReason,
        name: String,
        card: CardId,
        was_attacking: bool,
        was_token: bool,
        was_tapped: bool,
        was_enchanted: Option<CardId>,
        was_equipped: Option<CardId>,
        had_counters: IndexMap<Counter, usize>,
        turn: usize,
    },
    SpellResolved {
        spell: CardId,
        controller: Controller,
    },
    AbilityResolved {
        ability: AbilityId,
        controller: Controller,
    },
    TriggerResolved {
        source: CardId,
        controller: Controller,
    },
}

impl LogEntry {
    pub(crate) fn left_battlefield_passes_restrictions(
        &self,
        restrictions: &[Restriction],
    ) -> bool {
        match self {
            LogEntry::LeftBattlefield {
                was_attacking,
                was_tapped,
                ..
            } => {
                for restriction in restrictions.iter() {
                    match restriction {
                        Restriction::Attacking => {
                            if !was_attacking {
                                return false;
                            }
                        }
                        Restriction::Tapped => {
                            if !was_tapped {
                                return false;
                            }
                        }
                        _ => todo!(),
                    }
                }
            }

            _ => return false,
        }

        true
    }
}

#[derive(Debug, Resource, Default)]
pub struct Log {
    pub entries: Vec<(LogId, LogEntry)>,
    last_turn: usize,
}

impl Log {
    pub(crate) fn ability_resolved(db: &mut Database, id: LogId, ability: AbilityId) {
        let entry = LogEntry::AbilityResolved {
            ability,
            controller: ability.controller(db),
        };
        event!(Level::INFO, ?entry);
        db.resource_mut::<Self>().entries.push((id, entry))
    }

    pub(crate) fn spell_resolved(db: &mut Database, id: LogId, spell: CardId) {
        let entry = LogEntry::SpellResolved {
            spell,
            controller: spell.controller(db),
        };
        event!(Level::INFO, ?entry);
        db.resource_mut::<Self>().entries.push((id, entry))
    }

    pub(crate) fn trigger_resolved(db: &mut Database, id: LogId, trigger: TriggerId) {
        let entry = LogEntry::TriggerResolved {
            source: trigger.listener(db),
            controller: trigger.listener(db).controller(db),
        };
        event!(Level::INFO, ?entry);
        db.resource_mut::<Self>().entries.push((id, entry))
    }

    pub(crate) fn new_turn(db: &mut Database, player: Owner) {
        let mut log = db.resource_mut::<Self>();
        let entry = LogEntry::NewTurn { player };
        event!(Level::INFO, ?entry);
        log.entries.push((LogId::new(), entry));
        log.last_turn = log.entries.len();
    }

    pub fn since_last_turn(db: &Database) -> &[(LogId, LogEntry)] {
        let log = db.resource::<Self>();
        log.entries.as_slice().split_at(log.last_turn).1
    }

    pub(crate) fn current_session(db: &Database) -> &[(LogId, LogEntry)] {
        let log = db.resource::<Self>();

        let current = LogId::current();
        if let Some(pos) = log.entries.iter().rev().position(|(id, _)| *id != current) {
            log.entries.split_at(pos + 1).1
        } else {
            &[]
        }
    }

    pub(crate) fn left_battlefield(
        db: &mut Database,
        id: LogId,
        reason: LeaveReason,
        card: CardId,
    ) {
        let modified_by = card.modified_by(db);
        let entry = LogEntry::LeftBattlefield {
            reason,
            name: card.name(db),
            card,
            was_attacking: card.attacking(db),
            was_token: card.is_token(db),
            was_tapped: card.tapped(db),
            was_enchanted: modified_by
                .iter()
                .copied()
                .find(|card| card.aura(db).is_some()),
            was_equipped: modified_by.iter().copied().find(|card| {
                card.activated_abilities(db).into_iter().any(|ability| {
                    ability
                        .effects(db)
                        .into_iter()
                        .any(|effect| effect.effect.is_equip())
                })
            }),
            had_counters: CounterId::all_counters_on(db, card),
            turn: current_turn(db).turn,
        };

        event!(Level::INFO, ?entry);
        db.resource_mut::<Self>().entries.push((id, entry));
    }
}
