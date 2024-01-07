use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use tracing::Level;

use crate::{
    counters::Counter,
    effects::EffectBehaviors,
    in_play::{CardId, Database},
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

#[derive(Debug, Clone, Copy)]
pub enum LeaveReason {
    Exiled,
    PutIntoGraveyard,
    ReturnedToHand,
    ReturnedToLibrary,
}

#[derive(Debug, Clone)]
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
        had_counters: HashMap<Counter, usize>,
        turn: usize,
    },
    SpellResolved {
        spell: CardId,
        controller: Controller,
    },
    AbilityResolved {
        controller: Controller,
    },
    Tapped {
        card: CardId,
    },
    Cast {
        card: CardId,
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

#[derive(Debug, Default)]
pub struct Log {
    pub entries: Vec<(LogId, LogEntry)>,
    last_turn: usize,
}

impl Log {
    pub(crate) fn ability_resolved(db: &mut Database, id: LogId, source: CardId) {
        let entry = LogEntry::AbilityResolved {
            controller: db[source].controller,
        };
        event!(Level::INFO, ?entry);
        db.log.entries.push((id, entry))
    }

    pub(crate) fn spell_resolved(db: &mut Database, id: LogId, spell: CardId) {
        let entry = LogEntry::SpellResolved {
            spell,
            controller: db[spell].controller,
        };
        event!(Level::INFO, ?entry);
        db.log.entries.push((id, entry))
    }

    pub(crate) fn new_turn(db: &mut Database, player: Owner) {
        let entry = LogEntry::NewTurn { player };
        event!(Level::INFO, ?entry);
        db.log.entries.push((LogId::new(), entry));
        db.log.last_turn = db.log.entries.len();
    }

    pub fn since_last_turn(db: &Database) -> &[(LogId, LogEntry)] {
        db.log.entries.as_slice().split_at(db.log.last_turn).1
    }

    pub(crate) fn current_session(db: &Database) -> &[(LogId, LogEntry)] {
        let current = LogId::current();
        if let Some(pos) = db
            .log
            .entries
            .iter()
            .rev()
            .position(|(id, _)| *id != current)
        {
            db.log.entries.split_at(pos + 1).1
        } else {
            &[]
        }
    }

    pub(crate) fn tapped(db: &mut Database, id: LogId, card: CardId) {
        let entry = LogEntry::Tapped { card };

        event!(Level::INFO, ?entry);
        db.log.entries.push((id, entry));
    }

    pub(crate) fn cast(db: &mut Database, id: LogId, card: CardId) {
        let entry = LogEntry::Cast { card };
        db.log.entries.push((id, entry));
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
            name: card.faceup_face(db).name.clone(),
            card,
            was_attacking: db[card].attacking.is_some(),
            was_token: db[card].token,
            was_tapped: card.tapped(db),
            was_enchanted: modified_by
                .iter()
                .copied()
                .find(|card| card.faceup_face(db).enchant.is_some()),
            was_equipped: modified_by.iter().copied().find(|card| {
                db[*card]
                    .modified_activated_abilities
                    .iter()
                    .any(|(_, ability)| {
                        ability
                            .effects
                            .iter()
                            .any(|effect| effect.effect.is_equip())
                    })
            }),
            had_counters: db[card].counters.clone(),
            turn: db.turn.turn_count,
        };

        event!(Level::INFO, ?entry);
        db.log.entries.push((id, entry));
    }
}
