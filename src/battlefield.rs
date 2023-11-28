use std::collections::HashSet;

use crate::card::{Ability, PlayedCard};

#[derive(Debug)]
pub struct Permanent {
    pub card: PlayedCard,
    pub tapped: bool,
}

#[derive(Debug, Default)]
pub struct Battlefield {
    pub permanents: Vec<Permanent>,

    pub effects: HashSet<Ability>,
}
impl Battlefield {
    pub(crate) fn add(&mut self, played: PlayedCard) {
        for ability in played.card.abilities.iter() {
            match ability {
                a @ Ability::GreenCannotBeCountered { .. } => {
                    self.effects.insert(a.clone());
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
