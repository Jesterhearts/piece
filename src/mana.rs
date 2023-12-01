use crate::{card::Color, protogen};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Mana {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
    Generic(usize),
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
