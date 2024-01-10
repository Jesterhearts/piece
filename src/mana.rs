use crate::protogen::{self, color::Color, cost::ManaCost, mana::Mana};

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash, strum::AsRefStr)]
pub(crate) enum ManaRestriction {
    ActivateAbility,
    ArtifactSpellOrAbility,
    None,
}

impl Mana {
    pub(crate) fn push_mana_symbol(self, result: &mut String) {
        match self {
            Mana::WHITE => result.push('\u{e600}'),
            Mana::BLUE => result.push('\u{e601}'),
            Mana::BLACK => result.push('\u{e602}'),
            Mana::RED => result.push('\u{e603}'),
            Mana::GREEN => result.push('\u{e604}'),
            Mana::COLORLESS => result.push('\u{e904}'),
        }
    }
}

impl ManaCost {
    pub(crate) fn color(&self) -> Color {
        match self {
            ManaCost::WHITE => Color::WHITE,
            ManaCost::BLUE => Color::BLUE,
            ManaCost::BLACK => Color::BLACK,
            ManaCost::RED => Color::RED,
            ManaCost::GREEN => Color::GREEN,
            ManaCost::COLORLESS => Color::COLORLESS,
            ManaCost::GENERIC => Color::COLORLESS,
            ManaCost::X => Color::COLORLESS,
            ManaCost::TWO_X => Color::COLORLESS,
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
            protogen::mana::mana_restriction::Restriction::ActivateAbility(_) => {
                Self::ActivateAbility
            }
            protogen::mana::mana_restriction::Restriction::ArtifactSpellOrAbility(_) => {
                Self::ArtifactSpellOrAbility
            }
        }
    }
}
