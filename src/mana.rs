use anyhow::anyhow;
use bevy_ecs::component::Component;

use crate::{card::Color, protogen};

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, Copy, Hash, strum::EnumIter)]
pub enum Mana {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
}

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, Copy, Hash)]
pub enum ManaCost {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
    Generic(usize),
    X,
    TwoX,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash, strum::AsRefStr, Component)]
pub enum ManaRestriction {
    None,
    ArtifactSpellOrAbility,
}

impl Mana {
    pub fn push_mana_symbol(self, result: &mut String) {
        match self {
            Mana::White => result.push('🔆'),
            Mana::Blue => result.push('💧'),
            Mana::Black => result.push('💀'),
            Mana::Red => result.push('🔺'),
            Mana::Green => result.push('🌳'),
            Mana::Colorless => result.push('⟡'),
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Mana::White => Color::White,
            Mana::Blue => Color::Blue,
            Mana::Black => Color::Black,
            Mana::Red => Color::Red,
            Mana::Green => Color::Green,
            Mana::Colorless => Color::Colorless,
        }
    }
}

impl ManaCost {
    pub fn push_mana_symbol(self, result: &mut String) {
        match self {
            ManaCost::White => result.push('🔆'),
            ManaCost::Blue => result.push('💧'),
            ManaCost::Black => result.push('💀'),
            ManaCost::Red => result.push('🔺'),
            ManaCost::Green => result.push('🌳'),
            ManaCost::Colorless => result.push('⟡'),
            ManaCost::Generic(count) => result.push_str(&format!("{}", count)),
            ManaCost::X => result.push('X'),
            ManaCost::TwoX => result.push_str("XX"),
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ManaCost::White => Color::White,
            ManaCost::Blue => Color::Blue,
            ManaCost::Black => Color::Black,
            ManaCost::Red => Color::Red,
            ManaCost::Green => Color::Green,
            ManaCost::Colorless => Color::Colorless,
            ManaCost::Generic(_) => Color::Colorless,
            ManaCost::X => Color::Colorless,
            ManaCost::TwoX => Color::Colorless,
        }
    }
}

impl TryFrom<&protogen::cost::ManaCost> for ManaCost {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::ManaCost) -> Result<Self, Self::Error> {
        value
            .cost
            .as_ref()
            .ok_or_else(|| anyhow!("Expected cost to have a cost specified"))
            .and_then(ManaCost::try_from)
    }
}

impl TryFrom<&protogen::cost::mana_cost::Cost> for ManaCost {
    type Error = anyhow::Error;
    fn try_from(value: &protogen::cost::mana_cost::Cost) -> Result<Self, Self::Error> {
        match value {
            protogen::cost::mana_cost::Cost::White(_) => Ok(Self::White),
            protogen::cost::mana_cost::Cost::Blue(_) => Ok(Self::Blue),
            protogen::cost::mana_cost::Cost::Black(_) => Ok(Self::Black),
            protogen::cost::mana_cost::Cost::Red(_) => Ok(Self::Red),
            protogen::cost::mana_cost::Cost::Green(_) => Ok(Self::Green),
            protogen::cost::mana_cost::Cost::Colorless(_) => Ok(Self::Colorless),
            protogen::cost::mana_cost::Cost::Generic(generic) => {
                Ok(Self::Generic(usize::try_from(generic.count)?))
            }
            protogen::cost::mana_cost::Cost::X(_) => Ok(Self::X),
            protogen::cost::mana_cost::Cost::Twox(_) => Ok(Self::TwoX),
        }
    }
}

impl TryFrom<&protogen::mana::Mana> for Mana {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::mana::Mana) -> Result<Self, Self::Error> {
        value
            .mana
            .as_ref()
            .ok_or_else(|| anyhow!("Expected mana to have a mana field specified"))
            .map(Self::from)
    }
}

impl From<&protogen::mana::mana::Mana> for Mana {
    fn from(value: &protogen::mana::mana::Mana) -> Self {
        match value {
            protogen::mana::mana::Mana::White(_) => Self::White,
            protogen::mana::mana::Mana::Blue(_) => Self::Blue,
            protogen::mana::mana::Mana::Black(_) => Self::Black,
            protogen::mana::mana::Mana::Red(_) => Self::Red,
            protogen::mana::mana::Mana::Green(_) => Self::Green,
            protogen::mana::mana::Mana::Colorless(_) => Self::Colorless,
        }
    }
}
