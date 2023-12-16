use std::{collections::HashSet, sync::atomic::Ordering};

use bevy_ecs::{component::Component, entity::Entity, query::With};
use derive_more::From;
use itertools::Itertools;

use crate::{
    abilities::TriggerListeners,
    card::OracleText,
    effects::{AnyEffect, Effects},
    in_play::{Active, CardId, Database, TriggerInStack, NEXT_STACK_SEQ},
    stack::{ActiveTarget, Settled, Stack, Targets},
    triggers::Location,
    types::Types,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Component)]
pub struct TriggerId(Entity);

impl TriggerId {
    pub fn update_stack_seq(self, db: &mut Database) {
        db.triggers.get_mut::<TriggerInStack>(self.0).unwrap().seq =
            NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed);
    }

    pub fn move_to_stack(self, db: &mut Database, source: CardId, targets: Vec<ActiveTarget>) {
        if Stack::split_second(db) {
            return;
        }

        db.triggers.spawn((
            TriggerInStack {
                seq: NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed),
                source,
                trigger: self,
            },
            Targets(targets),
        ));
    }

    pub fn remove_from_stack(self, db: &mut Database) {
        db.triggers.despawn(self.0);
    }

    pub fn location_from(self, db: &mut Database) -> Location {
        db.triggers.get::<Location>(self.0).copied().unwrap()
    }

    pub fn for_types(self, db: &mut Database) -> Types {
        db.triggers.get::<Types>(self.0).cloned().unwrap()
    }

    pub fn listeners(self, db: &mut Database) -> HashSet<CardId> {
        db.triggers
            .get::<TriggerListeners>(self.0)
            .cloned()
            .map(|l| l.0)
            .unwrap()
    }

    pub fn effects(self, db: &mut Database) -> Vec<AnyEffect> {
        db.triggers
            .get::<Effects>(self.0)
            .cloned()
            .unwrap_or_default()
            .0
    }

    pub fn active_triggers_of_source<Source: Component>(db: &mut Database) -> Vec<TriggerId> {
        let mut results = vec![];
        let mut of_type = db
            .triggers
            .query_filtered::<Entity, (With<Source>, With<Active>)>();

        for id in of_type.iter(&db.triggers) {
            results.push(Self(id));
        }

        results
    }

    pub fn activate_all_for_card(db: &mut Database, cardid: CardId) {
        let entities = Self::all_for_card(db, cardid);

        for entity in entities {
            db.triggers.entity_mut(entity.0).insert(Active);
        }
    }

    pub fn all_for_card(db: &mut Database, cardid: CardId) -> Vec<TriggerId> {
        db.triggers
            .query::<(Entity, &TriggerListeners)>()
            .iter(&db.triggers)
            .filter_map(|(entity, listeners)| {
                if listeners.contains(&cardid) {
                    Some(Self(entity))
                } else {
                    None
                }
            })
            .collect_vec()
    }

    pub fn unsubscribe_all_for_card(db: &mut Database, cardid: CardId) {
        for mut listeners in db
            .triggers
            .query::<&mut TriggerListeners>()
            .iter_mut(&mut db.triggers)
        {
            listeners.remove(&cardid);
        }
    }

    pub fn deactivate_all_for_card(db: &mut Database, cardid: CardId) {
        let entities = db
            .triggers
            .query_filtered::<(Entity, &TriggerListeners), With<Active>>()
            .iter(&db.triggers)
            .filter_map(|(entity, listeners)| {
                if listeners.contains(&cardid) {
                    Some(entity)
                } else {
                    None
                }
            })
            .collect_vec();

        for entity in entities {
            db.triggers.entity_mut(entity).remove::<Active>();
        }
    }

    pub fn add_listener(self, db: &mut Database, listener: CardId) {
        db.triggers
            .get_mut::<TriggerListeners>(self.0)
            .unwrap()
            .insert(listener);
    }

    pub fn text(self, db: &Database) -> String {
        db.triggers
            .get::<OracleText>(self.0)
            .cloned()
            .map(|text| text.0)
            .unwrap_or_default()
    }

    pub fn short_text(self, db: &Database) -> String {
        let mut text = self.text(db);
        if text.len() > 10 {
            text.truncate(10);
            text.push_str("...")
        }
        text
    }

    pub fn wants_targets(self, db: &mut Database, source: CardId) -> usize {
        let effects = self.effects(db);
        let controller = source.controller(db);
        effects
            .into_iter()
            .map(|effect| effect.into_effect(db, controller))
            .map(|effect| effect.wants_targets())
            .sum()
    }

    pub fn settle(self, db: &mut Database) {
        db.triggers.entity_mut(self.0).insert(Settled);
    }
}
