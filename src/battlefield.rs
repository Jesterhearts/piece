use std::collections::HashSet;

use crate::{
    card::{Ability, PlayedCard},
    player::PlayerRef,
};

#[derive(Debug, PartialEq)]
pub struct Permanent {
    pub card: PlayedCard,
    pub tapped: bool,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct StaticAbility {
    pub controller: PlayerRef,
    pub ability: Ability,
}

#[derive(Debug, Default)]
pub struct Battlefield {
    pub permanents: Vec<Permanent>,

    pub effects: HashSet<StaticAbility>,
}
impl Battlefield {
    pub(crate) fn add(&mut self, played: PlayedCard) {
        for ability in played.card.abilities.iter() {
            match ability {
                a @ Ability::GreenCannotBeCountered { .. } => {
                    self.effects.insert(StaticAbility {
                        controller: played.controller.clone(),
                        ability: a.clone(),
                    });
                }
                _ => {}
            }
        }
        self.permanents.push(Permanent {
            card: played,
            tapped: false,
        });
    }
}
