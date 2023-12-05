use bevy_ecs::{component::Component, entity::Entity, world::World};
use derive_more::{Deref, From};

use crate::mana::Mana;

#[derive(Debug, Clone, Copy, Default, Component)]
pub struct ManaPool {
    pub white_mana: usize,
    pub blue_mana: usize,
    pub black_mana: usize,
    pub red_mana: usize,
    pub green_mana: usize,
    pub colorless_mana: usize,
}

impl ManaPool {
    pub fn apply(&mut self, mana: Mana) {
        match mana {
            Mana::White => self.white_mana += 1,
            Mana::Blue => self.blue_mana += 1,
            Mana::Black => self.black_mana += 1,
            Mana::Red => self.red_mana += 1,
            Mana::Green => self.green_mana += 1,
            Mana::Colorless => self.colorless_mana += 1,
            Mana::Generic(count) => self.colorless_mana += count,
        }
    }

    pub fn spend(&mut self, mana: Mana) -> bool {
        match mana {
            Mana::White => {
                let Some(mana) = self.white_mana.checked_sub(1) else {
                    return false;
                };

                self.white_mana = mana;
            }
            Mana::Blue => {
                let Some(mana) = self.blue_mana.checked_sub(1) else {
                    return false;
                };

                self.blue_mana = mana;
            }
            Mana::Black => {
                let Some(mana) = self.black_mana.checked_sub(1) else {
                    return false;
                };

                self.black_mana = mana;
            }
            Mana::Red => {
                let Some(mana) = self.red_mana.checked_sub(1) else {
                    return false;
                };

                self.red_mana = mana;
            }
            Mana::Green => {
                let Some(mana) = self.green_mana.checked_sub(1) else {
                    return false;
                };

                self.green_mana = mana;
            }
            Mana::Colorless => {
                let Some(mana) = self.colorless_mana.checked_sub(1) else {
                    return false;
                };

                self.colorless_mana = mana;
            }
            Mana::Generic(count) => {
                let copy = *self;

                for _ in 0..count {
                    let Some(mana) = self.max().checked_sub(1) else {
                        *self = copy;
                        return false;
                    };

                    *self.max() = mana;
                }
            }
        }

        true
    }

    #[cfg(test)]
    pub fn infinite(&mut self) {
        self.white_mana = usize::MAX;
        self.blue_mana = usize::MAX;
        self.black_mana = usize::MAX;
        self.red_mana = usize::MAX;
        self.green_mana = usize::MAX;
        self.colorless_mana = usize::MAX;
    }

    fn max(&mut self) -> &mut usize {
        [
            &mut self.white_mana,
            &mut self.blue_mana,
            &mut self.black_mana,
            &mut self.red_mana,
            &mut self.green_mana,
            &mut self.colorless_mana,
        ]
        .into_iter()
        .max()
        .unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component, Deref)]
pub struct Controller(Entity);

impl From<Owner> for Controller {
    fn from(value: Owner) -> Self {
        Self(value.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component, From, Deref)]
pub struct Owner(Entity);

impl Owner {
    pub fn new(world: &mut World) -> Self {
        let this = world.spawn(Player).insert(ManaPool::default()).id();
        Self(this)
    }
}
