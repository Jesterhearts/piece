use std::sync::atomic::Ordering;

use bevy_ecs::{component::Component, entity::Entity, query::With};
use derive_more::From;
use itertools::Itertools;

use crate::{
    abilities::{TriggerListener, TriggeredAbility},
    card::OracleText,
    controller::ControllerRestriction,
    effects::{AnyEffect, Effects},
    in_play::{Active, CardId, Database, Temporary, TriggerInStack, NEXT_STACK_SEQ},
    stack::{ActiveTarget, Settled, Stack, Targets},
    targets::Restrictions,
    triggers::{trigger_source, Location, TriggerSource},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Component)]
pub struct TriggerId(Entity);

impl TriggerId {
    pub fn upload(
        db: &mut Database,
        ability: &TriggeredAbility,
        card: CardId,
        temporary: bool,
    ) -> Self {
        debug!(
            "Uploading triggered ability for {}: {:?}",
            card.name(db),
            ability
        );
        let mut entity = db.triggers.spawn((
            TriggerListener(card),
            ability.trigger.from,
            Effects(ability.effects.clone()),
            Restrictions(ability.trigger.restrictions.clone()),
            OracleText(ability.oracle_text.clone()),
            ability.trigger.controller,
        ));

        if temporary {
            entity.insert(Temporary);
        }

        match ability.trigger.trigger {
            TriggerSource::PutIntoGraveyard => {
                entity.insert(trigger_source::PutIntoGraveyard);
            }
            TriggerSource::EntersTheBattlefield => {
                entity.insert(trigger_source::EntersTheBattlefield);
            }
            TriggerSource::ExiledDuringCraft => {
                entity.insert(trigger_source::ExiledDuringCraft);
            }
            TriggerSource::Cast => {
                entity.insert(trigger_source::Cast);
            }
            TriggerSource::StartOfCombat => {
                entity.insert(trigger_source::StartOfCombat);
            }
            TriggerSource::Tapped => {
                entity.insert(trigger_source::Tapped);
            }
            TriggerSource::Attacks => {
                entity.insert(trigger_source::Attacks);
            }
            TriggerSource::EndStep => {
                entity.insert(trigger_source::EndStep);
            }
        }

        Self(entity.id())
    }

    pub fn update_stack_seq(self, db: &mut Database) {
        db.triggers.get_mut::<TriggerInStack>(self.0).unwrap().seq =
            NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed);
    }

    pub fn move_to_stack(
        self,
        db: &mut Database,
        listener: CardId,
        targets: Vec<Vec<ActiveTarget>>,
    ) {
        if Stack::split_second(db) {
            return;
        }

        db.triggers.spawn((
            TriggerInStack {
                seq: NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed),
                source: listener,
                trigger: self,
            },
            Targets(targets),
        ));
    }

    pub fn remove_from_stack(self, db: &mut Database) {
        db.triggers.despawn(self.0);
    }

    pub fn location_from(self, db: &Database) -> Location {
        db.triggers.get::<Location>(self.0).copied().unwrap()
    }

    pub fn restrictions(self, db: &Database) -> Restrictions {
        db.triggers.get::<Restrictions>(self.0).cloned().unwrap()
    }

    pub fn listener(self, db: &Database) -> CardId {
        db.triggers
            .get::<TriggerListener>(self.0)
            .map(|l| l.0)
            .unwrap()
    }

    pub fn effects(self, db: &Database) -> Vec<AnyEffect> {
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
        let triggers = cardid.triggers(db);

        debug!("All triggers for card {}: {:?}", cardid.name(db), triggers);
        for trigger in triggers {
            db.triggers.entity_mut(trigger.0).insert(Active);
        }
    }

    pub fn unsubscribe_all_for_card(db: &mut Database, cardid: CardId) {
        let triggers = cardid.triggers(db);

        for trigger in triggers {
            db.triggers
                .entity_mut(trigger.0)
                .remove::<TriggerListener>();
        }
    }

    pub fn deactivate_all_for_card(db: &mut Database, cardid: CardId) {
        let triggers = cardid.triggers(db);

        for trigger in triggers {
            if db.triggers.get::<Temporary>(trigger.0).is_some() {
                db.triggers.despawn(trigger.0);
            } else {
                db.triggers.entity_mut(trigger.0).remove::<Active>();
            }
        }
    }

    pub fn add_listener(self, db: &mut Database, listener: CardId) {
        db.triggers
            .entity_mut(self.0)
            .insert(TriggerListener(listener));
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

    pub fn needs_targets(self, db: &mut Database, source: CardId) -> Vec<usize> {
        let effects = self.effects(db);
        let controller = source.controller(db);
        effects
            .into_iter()
            .map(|effect| effect.into_effect(db, controller))
            .map(|effect| effect.needs_targets())
            .collect_vec()
    }

    pub fn settle(self, db: &mut Database) {
        db.triggers.entity_mut(self.0).insert(Settled);
    }

    pub fn controller_restriction(self, db: &Database) -> ControllerRestriction {
        db.triggers
            .get::<ControllerRestriction>(self.0)
            .copied()
            .unwrap()
    }
}
