use bevy_ecs::{component::Component, system::Resource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct BattlefieldId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct GraveyardId(usize);

#[derive(Debug, Default, Resource)]
pub struct Battlefield {
    next_id: usize,
}

impl Battlefield {
    pub fn next_graveyard_id(&mut self) -> GraveyardId {
        let id = self.next_id;
        self.next_id += 1;
        GraveyardId(id)
    }
}
