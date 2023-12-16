use std::sync::atomic::Ordering;

use bevy_ecs::{component::Component, entity::Entity, query::With};
use itertools::Itertools;

use crate::{
    effects::{replacing, AnyEffect, Effects, ReplacementEffect, Replacing},
    in_play::{Active, CardId, Database, NEXT_REPLACEMENT_SEQ},
    targets::{Restriction, Restrictions},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct ReplacementSeq(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplacementEffectId(Entity);

impl ReplacementEffectId {
    pub fn watching<Replacing: Component>(db: &mut Database) -> Vec<Self> {
        db.replacement_effects
            .query_filtered::<(Entity, &ReplacementSeq), (With<Active>, With<Replacing>)>()
            .iter(&db.replacement_effects)
            .sorted_by_key(|(_, seq)| *seq)
            .map(|(e, _)| Self(e))
            .collect_vec()
    }

    pub fn upload_replacement_effect(
        db: &mut Database,
        effect: &ReplacementEffect,
        source: CardId,
    ) -> Self {
        let mut entity = db.replacement_effects.spawn((
            source,
            Restrictions(effect.restrictions.clone()),
            Effects(effect.effects.clone()),
        ));

        match effect.replacing {
            Replacing::Draw => {
                entity.insert(replacing::Draw);
            }
            Replacing::Etb => {
                entity.insert(replacing::Etb);
            }
        }

        Self(entity.id())
    }

    pub fn activate_all_for_card(db: &mut Database, card: CardId) {
        let all = db
            .replacement_effects
            .query::<(Entity, &CardId)>()
            .iter(&db.replacement_effects)
            .filter_map(|(e, watcher)| if *watcher == card { Some(e) } else { None })
            .collect_vec();

        for entity in all {
            db.replacement_effects
                .entity_mut(entity)
                .insert(Active)
                .insert(ReplacementSeq(
                    NEXT_REPLACEMENT_SEQ.fetch_add(1, Ordering::Relaxed),
                ));
        }
    }

    pub fn deactivate_all_for_card(db: &mut Database, card: CardId) {
        let all = db
            .replacement_effects
            .query::<(Entity, &CardId)>()
            .iter(&db.replacement_effects)
            .filter_map(|(e, watcher)| if *watcher == card { Some(e) } else { None })
            .collect_vec();

        for entity in all {
            db.replacement_effects.entity_mut(entity).remove::<Active>();
        }
    }

    pub fn restrictions(self, db: &mut Database) -> Vec<Restriction> {
        db.replacement_effects
            .get::<Restrictions>(self.0)
            .unwrap()
            .0
            .clone()
    }

    pub fn effects(self, db: &mut Database) -> Vec<AnyEffect> {
        db.replacement_effects
            .get::<Effects>(self.0)
            .unwrap()
            .0
            .clone()
    }

    pub fn source(self, db: &mut Database) -> CardId {
        db.replacement_effects
            .get::<CardId>(self.0)
            .copied()
            .unwrap()
    }
}