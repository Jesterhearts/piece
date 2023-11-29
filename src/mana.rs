use serde::{Deserialize, Serialize};

use crate::card::Color;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum ManaGain {
    Specific(Vec<Mana>),
    Choice(Vec<Vec<Mana>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum Mana {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
    Generic(usize),
}

impl Mana {
    pub fn color(&self) -> Color {
        match self {
            Mana::White => Color::White,
            Mana::Blue => Color::Blue,
            Mana::Black => Color::Black,
            Mana::Red => Color::Red,
            Mana::Green => Color::Green,
            Mana::Colorless => Color::Colorless,
            Mana::Generic(_) => Color::Colorless,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub enum AdditionalCost {
    SacrificeThis,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    #[serde(default)]
    pub mana: Vec<Mana>,
    #[serde(default)]
    pub tap: bool,
    #[serde(default)]
    pub additional_costs: Vec<AdditionalCost>,
}
