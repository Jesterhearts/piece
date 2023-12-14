use anyhow::anyhow;

use crate::{card::Color, protogen};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
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
    pub fn push_mana_symbol(self, result: &mut String) {
        match self {
            Mana::White => result.push('🔆'),
            Mana::Blue => result.push('💧'),
            Mana::Black => result.push('💀'),
            Mana::Red => result.push('🔺'),
            Mana::Green => result.push('🌳'),
            Mana::Colorless => result.push('⟡'),
            Mana::Generic(count) => result.push_str(&format!("{}", count)),
        }
    }
}

impl TryFrom<&protogen::cost::ManaCost> for Mana {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::ManaCost) -> Result<Self, Self::Error> {
        value
            .cost
            .as_ref()
            .ok_or_else(|| anyhow!("Expected cost to have a cost specified"))
            .and_then(Mana::try_from)
    }
}

impl TryFrom<&protogen::cost::mana_cost::Cost> for Mana {
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
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::mana::mana::Mana> for Mana {
    type Error = anyhow::Error;
    fn try_from(value: &protogen::mana::mana::Mana) -> Result<Self, Self::Error> {
        match value {
            protogen::mana::mana::Mana::White(_) => Ok(Self::White),
            protogen::mana::mana::Mana::Blue(_) => Ok(Self::Blue),
            protogen::mana::mana::Mana::Black(_) => Ok(Self::Black),
            protogen::mana::mana::Mana::Red(_) => Ok(Self::Red),
            protogen::mana::mana::Mana::Green(_) => Ok(Self::Green),
            protogen::mana::mana::Mana::Colorless(_) => Ok(Self::Colorless),
            protogen::mana::mana::Mana::Generic(generic) => {
                Ok(Self::Generic(usize::try_from(generic.count)?))
            }
        }
    }
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
