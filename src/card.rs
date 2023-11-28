use std::{
    cell::RefCell,
    collections::{BTreeSet, HashSet},
    rc::Rc,
};

use serde::{Deserialize, Serialize};

use crate::{
    activated_ability,
    battlefield::Battlefield,
    mana::{Cost, Mana},
    player::Player,
    stack::{ActiveTarget, EntryType, Stack},
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub enum Controller {
    You,
    Opponent,
    Any,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub enum Target {
    Spell {
        controller: Controller,
        #[serde(default)]
        types: Vec<Type>,
        #[serde(default)]
        subtypes: Vec<Subtype>,
    },
    Creature {
        types: BTreeSet<Subtype>,
    },
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub enum Effect {
    CounterSpell {
        target: Target,
    },
    CannotBeCountered,
    GainMana(Vec<Mana>),
    ModifyBasePT {
        targets: Vec<Target>,
        base_power: i32,
        base_toughness: i32,
    },
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub enum Ability {
    GreenCannotBeCountered { controller: Controller },
    Activated(activated_ability::Ability),
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub enum LandType {
    Plains,
    Island,
    Swamp,
    Mountain,
    Forest,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub enum Type {
    BasicLand(LandType),
    Land { types: Vec<LandType> },
    Instant,
    Sorcery,
    Creature,
    Artifact,
    Enchantment,
    Battle,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone, PartialOrd, Ord)]
pub enum Subtype {
    Elf,
    Shaman,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum Targets {
    Spells,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Card {
    pub name: String,
    pub cost: Cost,
    #[serde(default)]
    pub oracle_text: String,
    #[serde(default)]
    pub effects: HashSet<Effect>,
    #[serde(default)]
    pub abilities: HashSet<Ability>,
    pub ty: Type,
    #[serde(default)]
    pub subtypes: HashSet<Subtype>,
    #[serde(default)]
    pub targets: HashSet<Targets>,
}

impl Card {
    pub fn color(&self) -> HashSet<Color> {
        let mut colors = HashSet::default();
        for mana in self.cost.mana.iter() {
            let color = mana.color();
            colors.insert(color);
        }

        colors
    }

    pub fn color_identity(&self) -> HashSet<Color> {
        let mut identity = self.color();

        for ability in self.abilities.iter() {
            if let Ability::Activated(activated) = ability {
                for mana in activated.cost.mana.iter() {
                    let color = mana.color();
                    identity.insert(color);
                }
            }
        }

        identity
    }

    pub fn uses_stack(&self) -> bool {
        !self.is_land()
    }

    pub fn requires_target(&self) -> bool {
        !self.targets.is_empty()
    }

    pub fn is_land(&self) -> bool {
        matches!(self.ty, Type::BasicLand(_) | Type::Land { .. })
    }

    pub fn valid_targets(
        &self,
        battlefield: &Battlefield,
        stack: &Stack,
        caster: &Player,
    ) -> HashSet<ActiveTarget> {
        let mut targets = HashSet::default();

        for target in self.targets.iter() {
            match target {
                Targets::Spells => {
                    for effect in self.effects.iter() {
                        match effect {
                            Effect::CounterSpell { target } => {
                                for (index, spell) in stack.spells.iter() {
                                    match &spell.ty {
                                        EntryType::Card(card) => {
                                            if card.card.can_be_countered(
                                                battlefield,
                                                caster,
                                                &card.controller.borrow(),
                                                target,
                                            ) {
                                                targets.insert(ActiveTarget::Stack { id: *index });
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        targets
    }

    pub fn can_be_countered(
        &self,
        battlefield: &Battlefield,
        caster: &Player,
        this_controller: &Player,
        target: &Target,
    ) -> bool {
        for effect in self.effects.iter() {
            match effect {
                Effect::CannotBeCountered => {
                    return false;
                }
                _ => {}
            }
        }

        match target {
            Target::Spell {
                controller,
                types,
                subtypes,
            } => {
                match controller {
                    Controller::You => {
                        if caster.id != this_controller.id {
                            return false;
                        }
                    }
                    Controller::Opponent => {
                        if caster.id == this_controller.id {
                            return false;
                        }
                    }
                    Controller::Any => {}
                };

                if !types.is_empty() && !self.types_intersect(types) {
                    return false;
                }

                if !self.subtypes_match(subtypes) {
                    return false;
                }
            }
            _ => return false,
        }

        for effect in battlefield.effects.iter() {
            match effect {
                Ability::GreenCannotBeCountered { controller } => {
                    if self.color().contains(&Color::Green) {
                        match controller {
                            Controller::You => {
                                if caster.id != this_controller.id {
                                    return false;
                                }
                            }
                            Controller::Opponent => {
                                if caster.id == this_controller.id {
                                    return false;
                                }
                            }
                            Controller::Any => {
                                return false;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        true
    }
    pub fn types_intersect(&self, types: &[Type]) -> bool {
        types.iter().any(|ty| self.ty == *ty)
    }

    pub fn subtypes_match(&self, subtypes: &[Subtype]) -> bool {
        subtypes.iter().all(|ty| self.subtypes.contains(ty))
    }
}

#[derive(Debug, Clone)]
pub struct PlayedCard {
    pub card: Rc<Card>,
    pub controller: Rc<RefCell<Player>>,
    pub owner: Rc<RefCell<Player>>,
}
