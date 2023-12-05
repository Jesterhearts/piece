use bevy_ecs::{component::Component, entity::Entity};

use crate::effects::ActivatedAbilityEffect;

#[derive(Debug, Component)]
pub struct ActiveAbility {
    pub source: Entity,
    pub effects: Vec<ActivatedAbilityEffect>,
}
