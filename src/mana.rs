use anyhow::anyhow;
use bevy_ecs::component::Component;

use crate::{card::Color, protogen};

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, Copy, Hash, strum::EnumIter)]
pub(crate) enum Mana {
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
pub(crate) enum ManaRestriction {
    ArtifactSpellOrAbility,
    None,
}

impl Mana {
    pub(crate) fn push_mana_symbol(self, result: &mut String) {
        match self {
            Mana::White => result.push('\u{e600}'),
            Mana::Blue => result.push('\u{e601}'),
            Mana::Black => result.push('\u{e602}'),
            Mana::Red => result.push('\u{e603}'),
            Mana::Green => result.push('\u{e604}'),
            Mana::Colorless => result.push('\u{e904}'),
        }
    }

    pub(crate) fn color(&self) -> Color {
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
    pub(crate) fn push_mana_symbol(self, result: &mut String) {
        match self {
            ManaCost::White => result.push('\u{e600}'),
            ManaCost::Blue => result.push('\u{e601}'),
            ManaCost::Black => result.push('\u{e602}'),
            ManaCost::Red => result.push('\u{e603}'),
            ManaCost::Green => result.push('\u{e604}'),
            ManaCost::Colorless => result.push('\u{e904}'),
            ManaCost::Generic(count) => match count {
                0 => result.push('\u{e605}'),
                1 => result.push('\u{e606}'),
                2 => result.push('\u{e607}'),
                3 => result.push('\u{e608}'),
                4 => result.push('\u{e609}'),
                5 => result.push('\u{e60a}'),
                6 => result.push('\u{e60b}'),
                7 => result.push('\u{e60c}'),
                8 => result.push('\u{e60d}'),
                9 => result.push('\u{e60e}'),
                10 => result.push('\u{e60f}'),
                11 => result.push('\u{e610}'),
                12 => result.push('\u{e611}'),
                13 => result.push('\u{e612}'),
                14 => result.push('\u{e613}'),
                15 => result.push('\u{e614}'),
                16 => result.push('\u{e62a}'),
                17 => result.push('\u{e62b}'),
                18 => result.push('\u{e62c}'),
                19 => result.push('\u{e62d}'),
                20 => result.push('\u{e62e}'),
                _ => result.push_str(&format!("{}", count)),
            },
            ManaCost::X => result.push('\u{e615}'),
            ManaCost::TwoX => result.push_str("\u{e615}\u{e615}"),
        }
    }

    pub(crate) fn color(&self) -> Color {
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

impl From<&protogen::mana::ManaRestriction> for ManaRestriction {
    fn from(value: &protogen::mana::ManaRestriction) -> Self {
        value
            .restriction
            .as_ref()
            .map(Self::from)
            .unwrap_or(Self::None)
    }
}

impl From<&protogen::mana::mana_restriction::Restriction> for ManaRestriction {
    fn from(value: &protogen::mana::mana_restriction::Restriction) -> Self {
        match value {
            protogen::mana::mana_restriction::Restriction::ArtifactSpellOrAbility(_) => {
                Self::ArtifactSpellOrAbility
            }
        }
    }
}
