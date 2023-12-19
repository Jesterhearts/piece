use bevy_ecs::{component::Component, entity::Entity};
use derive_more::From;

use crate::in_play::{Database, Modifiers};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Component, From)]
pub struct AuraId(Entity);

impl AuraId {
    pub fn modifiers(self, db: &Database) -> Modifiers {
        db.auras.get::<Modifiers>(self.0).cloned().unwrap()
    }

    pub fn is_attached(self, db: &Database) -> bool {
        let modifiers = self.modifiers(db);
        for modifier in modifiers.iter() {
            if !modifier.modifying(db).is_empty() {
                return true;
            }
        }

        false
    }
}
