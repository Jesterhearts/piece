use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    activated_ability::Ability,
    battlefield::Battlefield,
    in_play::AllCards,
    mana::{Cost, ManaGain},
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

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone, Default)]
pub enum Controller {
    You,
    Opponent,
    #[default]
    Any,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub enum Target {
    Spell {
        #[serde(default)]
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
    GainMana {
        mana: ManaGain,
    },
    ModifyBasePT {
        targets: Vec<Target>,
        base_power: i32,
        base_toughness: i32,
    },
    ControllerDrawCards(usize),
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone, Copy)]
pub enum CastingModifier {
    CannotBeCountered,
    SplitSecond,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub enum StaticAbility {
    GreenCannotBeCountered { controller: Controller },
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

impl Type {
    fn is_permanent(&self) -> bool {
        match self {
            Type::BasicLand(_)
            | Type::Land { .. }
            | Type::Creature
            | Type::Artifact
            | Type::Enchantment
            | Type::Battle => true,
            Type::Sorcery | Type::Instant => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone, PartialOrd, Ord)]
pub enum Subtype {
    Bear,
    Elf,
    Shaman,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum Targets {
    Spells,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub name: String,
    pub cost: Cost,
    #[serde(default)]
    pub oracle_text: String,
    #[serde(default)]
    pub flavor_text: String,
    #[serde(default)]
    pub casting_modifiers: HashSet<CastingModifier>,
    #[serde(default)]
    pub effects: HashSet<Effect>,
    #[serde(default)]
    pub static_abilities: Vec<StaticAbility>,
    #[serde(default)]
    pub activated_abilities: Vec<Ability>,
    pub ty: Type,
    #[serde(default)]
    pub subtypes: HashSet<Subtype>,
    #[serde(default)]
    pub targets: HashSet<Targets>,
    pub power: Option<usize>,
    pub toughness: Option<usize>,
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

        for ability in self.activated_abilities.iter() {
            for mana in ability.cost.mana.iter() {
                let color = mana.color();
                identity.insert(color);
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
        cards: &AllCards,
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
                                for (index, spell) in stack.stack.iter() {
                                    match &spell.ty {
                                        EntryType::Card(card) => {
                                            let card = &cards[*card];
                                            if card.card.can_be_countered(
                                                cards,
                                                battlefield,
                                                caster,
                                                &card.controller.borrow(),
                                                target,
                                            ) {
                                                targets.insert(ActiveTarget::Stack { id: *index });
                                            }
                                        }
                                        EntryType::Effect(_) => {}
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
        cards: &AllCards,
        battlefield: &Battlefield,
        caster: &Player,
        this_controller: &Player,
        target: &Target,
    ) -> bool {
        for modifier in self.casting_modifiers.iter() {
            match modifier {
                CastingModifier::CannotBeCountered => {
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

        for (ability, controllers) in battlefield.static_abilities(cards).into_iter() {
            match &ability {
                StaticAbility::GreenCannotBeCountered { controller } => {
                    if self.color().contains(&Color::Green) {
                        match controller {
                            Controller::You => {
                                if controllers
                                    .into_iter()
                                    .any(|controller| controller == *this_controller)
                                {
                                    return false;
                                }
                            }
                            Controller::Opponent => {
                                if controllers
                                    .into_iter()
                                    .any(|controller| controller != *this_controller)
                                {
                                    return false;
                                }
                            }
                            Controller::Any => {
                                return false;
                            }
                        }
                    }
                }
            }
        }

        true
    }

    pub fn can_be_sacrificed(&self, _battlefield: &Battlefield) -> bool {
        // TODO: check battlefield for effects preventing sacrifice
        true
    }

    pub fn types_intersect(&self, types: &[Type]) -> bool {
        types.iter().any(|ty| self.ty == *ty)
    }

    pub fn subtypes_match(&self, subtypes: &[Subtype]) -> bool {
        subtypes.iter().all(|ty| self.subtypes.contains(ty))
    }

    pub fn is_permanent(&self) -> bool {
        self.ty.is_permanent()
    }
}
