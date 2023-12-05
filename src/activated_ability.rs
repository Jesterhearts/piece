use bevy_ecs::component::Component;

use crate::effects::ActivatedAbilityEffect;

#[derive(Debug, Component)]
pub struct ActiveAbility {
    pub effects: Vec<ActivatedAbilityEffect>,
}
