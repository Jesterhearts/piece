use std::collections::HashMap;

use tracing::Level;

use crate::{
    in_play::{ActivatedAbilityId, CardId, Database},
    player::{Controller, Owner},
    protogen::counters::Counter,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogId(usize);

impl LogId {
    fn new(db: &mut Database) -> Self {
        db.log.current_id += 1;
        Self(db.log.current_id)
    }

    pub(crate) fn current(db: &Database) -> Self {
        Self(db.log.current_id)
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
        had_counters: HashMap<Counter, u32>,
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
    Activated {
        card: CardId,
        ability: ActivatedAbilityId,
    },
    EtbOrTriggered {
        card: CardId,
    },
    CardChosen {
        card: CardId,
    },
    Discarded {
        card: CardId,
    },
}

#[derive(Debug, Default)]
pub struct Log {
    pub entries: Vec<(LogId, LogEntry)>,
    last_turn: usize,

    current_id: usize,
}

impl Log {
    pub(crate) fn card_chosen(db: &mut Database, chosen: CardId) {
        let entry = LogEntry::CardChosen { card: chosen };
        event!(Level::INFO, ?entry);
        db.log.entries.push((LogId::current(db), entry))
    }

    pub(crate) fn ability_resolved(db: &mut Database, source: CardId) {
        let entry = LogEntry::AbilityResolved {
            controller: db[source].controller,
        };
        let id = LogId::new(db);
        event!(Level::INFO, ?id, ?entry);
        db.log.entries.push((id, entry))
    }

    pub(crate) fn spell_resolved(db: &mut Database, spell: CardId) {
        let entry = LogEntry::SpellResolved {
            spell,
            controller: db[spell].controller,
        };
        let id = LogId::new(db);
        event!(Level::INFO, ?id, ?entry);
        db.log.entries.push((id, entry))
    }

    pub(crate) fn new_turn(db: &mut Database, player: Owner) {
        let entry = LogEntry::NewTurn { player };
        let id = LogId::new(db);
        event!(Level::INFO, ?id, ?entry);
        db.log.entries.push((id, entry));
        db.log.last_turn = db.log.entries.len();
    }

    pub fn since_last_turn(db: &Database) -> &[(LogId, LogEntry)] {
        db.log.entries.as_slice().split_at(db.log.last_turn).1
    }

    pub(crate) fn session(db: &Database, session: LogId) -> &[(LogId, LogEntry)] {
        if let Some(after) = db
            .log
            .entries
            .iter()
            .rev()
            .position(|(id, _)| *id == session)
        {
            if let Some(len) = db
                .log
                .entries
                .iter()
                .rev()
                .skip(after)
                .position(|(id, _)| *id != session)
            {
                let final_entry = db.log.entries.len() - after;
                let first_entry = db.log.entries.len() - (len + after);

                let entries = &db.log.entries.as_slice()[first_entry..final_entry];
                entries
            } else {
                &[]
            }
        } else {
            &[]
        }
    }

    pub(crate) fn tapped(db: &mut Database, card: CardId) {
        let entry = LogEntry::Tapped { card };

        let id = LogId::current(db);
        event!(Level::INFO, ?id, ?entry);
        db.log.entries.push((id, entry));
    }

    pub(crate) fn cast(db: &mut Database, card: CardId) {
        let entry = LogEntry::Cast { card };
        let id = LogId::new(db);
        event!(Level::INFO, ?id, ?entry);
        db.log.entries.push((id, entry));
    }

    pub(crate) fn activated(db: &mut Database, card: CardId, ability: ActivatedAbilityId) {
        let entry = LogEntry::Activated { card, ability };
        let id = LogId::new(db);
        event!(Level::INFO, ?id, ?entry);
        db.log.entries.push((id, entry));
    }

    pub(crate) fn etb_or_triggered(db: &mut Database, card: CardId) {
        let entry = LogEntry::EtbOrTriggered { card };
        let id = LogId::new(db);
        event!(Level::INFO, ?id, ?entry);
        db.log.entries.push((id, entry));
    }

    pub(crate) fn left_battlefield(db: &mut Database, reason: LeaveReason, card: CardId) {
        let entry = LogEntry::LeftBattlefield {
            reason,
            name: card.faceup_face(db).name.clone(),
            card,
            was_attacking: db[card].attacking.is_some(),
            was_token: db[card].token,
            was_tapped: card.tapped(db),
            had_counters: db[card].counters.clone(),
            turn: db.turn.turn_count,
        };

        let id = LogId::current(db);
        event!(Level::INFO, ?id, ?entry);
        db.log.entries.push((id, entry));
    }

    pub(crate) fn discarded(db: &mut Database, card: CardId) {
        let entry = LogEntry::Discarded { card };
        let id = LogId::current(db);
        event!(Level::INFO, ?id, ?entry);
        db.log.entries.push((id, entry));
    }
}
