use std::collections::HashSet;

use anyhow::anyhow;
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};

use crate::{
    abilities::{ActivatedAbility, ETBAbility, Enchant, StaticAbility, TriggeredAbility},
    cost::CastingCost,
    effects::{AnyEffect, Token, TokenCreature},
    in_play::{AbilityId, TriggerId},
    protogen,
    types::{Subtype, Type},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct SplitSecond;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct CannotBeCountered;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Component)]
pub struct TriggeredAbilities(pub Vec<TriggerId>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct Colors(pub HashSet<Color>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct AddColors(pub HashSet<Color>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
}

impl TryFrom<&protogen::color::Color> for Color {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::color::Color) -> Result<Self, Self::Error> {
        value
            .color
            .as_ref()
            .ok_or_else(|| anyhow!("Expected color to have a color set"))
            .map(Self::from)
    }
}

impl From<&protogen::color::color::Color> for Color {
    fn from(value: &protogen::color::color::Color) -> Self {
        match value {
            protogen::color::color::Color::White(_) => Self::White,
            protogen::color::color::Color::Blue(_) => Self::Blue,
            protogen::color::color::Color::Black(_) => Self::Black,
            protogen::color::color::Color::Red(_) => Self::Red,
            protogen::color::color::Color::Green(_) => Self::Green,
            protogen::color::color::Color::Colorless(_) => Self::Colorless,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeModifier {
    RemoveAll,
    Add(HashSet<Type>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtypeModifier {
    RemoveAll,
    Add(HashSet<Subtype>),
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub enum StaticAbilityModifier {
    RemoveAll,
    Add(StaticAbility),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub enum ActivatedAbilityModifier {
    RemoveAll,
    Add(AbilityId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub enum TriggeredAbilityModifier {
    RemoveAll,
    Add(TriggerId),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct Name(pub String);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut, Default,
)]
pub struct MarkedDamage(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct BasePower(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct BasePowerModifier(pub i32);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut, Default,
)]
pub struct AddPower(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct BaseToughness(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct BaseToughnessModifier(pub i32);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut, Default,
)]
pub struct AddToughness(pub i32);

#[derive(Debug, Clone, Copy, Component)]
pub struct Vigilance;

#[derive(Debug, Clone, Copy, Component)]
pub struct AddVigilance;

#[derive(Debug, Clone, Copy, Component)]
pub struct RemoveVigilance;

#[derive(Debug, Clone, Copy, Component)]
pub struct Flying;

#[derive(Debug, Clone, Copy, Component)]
pub struct AddFlying;

#[derive(Debug, Clone, Copy, Component)]
pub struct RemoveFlying;

#[derive(Debug, Clone, Copy, Component)]
pub struct Flash;

#[derive(Debug, Clone, Copy, Component)]
pub struct AddFlash;

#[derive(Debug, Clone, Copy, Component)]
pub struct RemoveFlash;

#[derive(Debug, Clone, Copy, Component)]
pub struct Hexproof;

#[derive(Debug, Clone, Copy, Component)]
pub struct AddHexproof;

#[derive(Debug, Clone, Copy, Component)]
pub struct RemoveHexproof;

#[derive(Debug, Clone, Copy, Component)]
pub struct Shroud;

#[derive(Debug, Clone, Copy, Component)]
pub struct AddShroud;

#[derive(Debug, Clone, Copy, Component)]
pub struct RemoveShroud;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Card {
    pub name: String,
    pub types: HashSet<Type>,
    pub subtypes: HashSet<Subtype>,

    pub cost: CastingCost,
    pub split_second: bool,
    pub cannot_be_countered: bool,

    pub colors: HashSet<Color>,

    pub oracle_text: String,

    pub enchant: Option<Enchant>,

    pub etb_abilities: Vec<ETBAbility>,
    pub effects: Vec<AnyEffect>,

    pub static_abilities: Vec<StaticAbility>,

    pub activated_abilities: Vec<ActivatedAbility>,

    pub triggered_abilities: Vec<TriggeredAbility>,

    pub power: Option<usize>,
    pub toughness: Option<usize>,

    pub vigilance: bool,
    pub flying: bool,
    pub flash: bool,
    pub hexproof: bool,
    pub shroud: bool,
}

impl TryFrom<protogen::card::Card> for Card {
    type Error = anyhow::Error;

    fn try_from(value: protogen::card::Card) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            cost: value
                .cost
                .as_ref()
                .ok_or_else(|| anyhow!("Expected a casting cost"))?
                .try_into()?,
            split_second: value.split_second,
            cannot_be_countered: value.cannot_be_countered,
            colors: value
                .colors
                .iter()
                .map(Color::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            oracle_text: value.oracle_text,
            enchant: value
                .enchant
                .as_ref()
                .map_or(Ok(None), |enchant| enchant.try_into().map(Some))?,
            etb_abilities: value
                .etb_abilities
                .iter()
                .map(ETBAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            static_abilities: value
                .static_abilities
                .iter()
                .map(StaticAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            activated_abilities: value
                .activated_abilities
                .iter()
                .map(ActivatedAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            triggered_abilities: value
                .triggered_abilities
                .iter()
                .map(TriggeredAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            power: value
                .power
                .map_or::<anyhow::Result<Option<usize>>, _>(Ok(None), |v| {
                    Ok(usize::try_from(v).map(Some)?)
                })?,
            toughness: value
                .toughness
                .map_or::<anyhow::Result<Option<usize>>, _>(Ok(None), |v| {
                    Ok(usize::try_from(v).map(Some)?)
                })?,
            vigilance: value.vigilance,
            flying: value.flying,
            flash: value.flash,
            hexproof: value.hexproof,
            shroud: value.shroud,
        })
    }
}

impl From<Token> for Card {
    fn from(value: Token) -> Self {
        match value {
            Token::Creature(TokenCreature {
                name,
                types,
                subtypes,
                colors,
                power,
                toughness,
            }) => Self {
                name,
                types,
                subtypes,
                colors,
                power: Some(power),
                toughness: Some(toughness),
                ..Default::default()
            },
        }
    }
}
