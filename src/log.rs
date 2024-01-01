use bevy_ecs::system::Resource;
use indexmap::IndexMap;

use crate::{
    effects::target_gains_counters::Counter,
    in_play::{current_turn, AbilityId, CardId, CounterId, Database, TriggerId},
    player::{Controller, Owner},
};

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

#[derive(Debug, Resource, Default)]
pub struct Log {
    pub entries: Vec<LogEntry>,
    last_turn: usize,
}

impl Log {
    pub(crate) fn ability_resolved(db: &mut Database, ability: AbilityId) {
        let entry = LogEntry::AbilityResolved {
            ability,
            controller: ability.controller(db),
        };
        db.resource_mut::<Self>().entries.push(entry)
    }

    pub(crate) fn spell_resolved(db: &mut Database, spell: CardId) {
        let entry = LogEntry::SpellResolved {
            spell,
            controller: spell.controller(db),
        };
        db.resource_mut::<Self>().entries.push(entry)
    }

    pub(crate) fn trigger_resolved(db: &mut Database, trigger: TriggerId) {
        let entry = LogEntry::TriggerResolved {
            source: trigger.listener(db),
            controller: trigger.listener(db).controller(db),
        };
        db.resource_mut::<Self>().entries.push(entry)
    }

    pub(crate) fn new_turn(db: &mut Database, player: Owner) {
        let mut log = db.resource_mut::<Self>();
        log.entries.push(LogEntry::NewTurn { player });
        log.last_turn = log.entries.len();
    }

    pub fn since_last_turn(db: &Database) -> &[LogEntry] {
        let log = db.resource::<Self>();
        log.entries.as_slice().split_at(log.last_turn).1
    }

    pub(crate) fn left_battlefield(db: &mut Database, reason: LeaveReason, card: CardId) {
        let modified_by = card.modified_by(db);
        let entry = LogEntry::LeftBattlefield {
            reason,
            name: card.name(db),
            card,
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
                        .any(|effect| effect.into_effect(db, card.controller(db)).is_equip())
                })
            }),
            had_counters: CounterId::all_counters_on(db, card),
            turn: current_turn(db).turn,
        };

        db.resource_mut::<Self>().entries.push(entry);
    }
}
