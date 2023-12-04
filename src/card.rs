use anyhow::anyhow;
use bevy_ecs::{component::Component, entity::Entity, system::Query};
use enumset::{EnumSet, EnumSetType};
use indexmap::IndexSet;

use crate::{
    abilities::{ActivatedAbility, ETBAbility, StaticAbility},
    cost::CastingCost,
    effects::SpellEffect,
    protogen,
    types::{Subtype, Type},
};

#[derive(Debug, Component)]
pub enum ModifyingTypeSet {
    Adding(EnumSet<Type>),
    RemovingAll,
    Subtracting(EnumSet<Type>),
}

#[derive(Debug, Component)]
pub enum ModifyingSubtypeSet {
    Adding(EnumSet<Subtype>),
    RemovingAll,
    Subtracting(EnumSet<Subtype>),
}

#[derive(Debug, Component)]
pub struct ModifyingTypes(pub IndexSet<Entity>);

impl ModifyingTypes {
    pub fn union(
        &self,
        base: &Card,
        query: &Query<&ModifyingTypeSet>,
    ) -> anyhow::Result<EnumSet<Type>> {
        let mut types = base.types;
        for entity in self.0.iter().copied() {
            let modifier = query.get(entity)?;
            match modifier {
                ModifyingTypeSet::Adding(adding) => {
                    types.insert_all(*adding);
                }
                ModifyingTypeSet::RemovingAll => types.clear(),
                ModifyingTypeSet::Subtracting(subtracting) => {
                    types.remove_all(*subtracting);
                }
            }
        }

        Ok(types)
    }
}

#[derive(Debug, Component)]
pub struct ModifyingSubtypes(pub IndexSet<Entity>);

impl ModifyingSubtypes {
    pub fn union(
        &self,
        base: &Card,
        query: &Query<&ModifyingSubtypeSet>,
    ) -> anyhow::Result<EnumSet<Subtype>> {
        let mut types = base.subtypes;
        for entity in self.0.iter().copied() {
            let modifier = query.get(entity)?;
            match modifier {
                ModifyingSubtypeSet::Adding(adding) => {
                    types.insert_all(*adding);
                }
                ModifyingSubtypeSet::RemovingAll => types.clear(),
                ModifyingSubtypeSet::Subtracting(subtracting) => {
                    types.remove_all(*subtracting);
                }
            }
        }

        Ok(types)
    }
}

#[derive(Debug, EnumSetType)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
}

impl TryFrom<&protogen::card::Color> for Color {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::card::Color) -> Result<Self, Self::Error> {
        value
            .color
            .as_ref()
            .ok_or_else(|| anyhow!("Expected color to have a color set"))
            .map(Self::from)
    }
}

impl From<&protogen::card::color::Color> for Color {
    fn from(value: &protogen::card::color::Color) -> Self {
        match value {
            protogen::card::color::Color::White(_) => Self::White,
            protogen::card::color::Color::Blue(_) => Self::Blue,
            protogen::card::color::Color::Black(_) => Self::Black,
            protogen::card::color::Color::Red(_) => Self::Red,
            protogen::card::color::Color::Green(_) => Self::Green,
            protogen::card::color::Color::Colorless(_) => Self::Colorless,
        }
    }
}

#[derive(Debug, EnumSetType)]
pub enum CastingModifier {
    CannotBeCountered,
    SplitSecond,
}

impl TryFrom<&protogen::card::CastingModifier> for CastingModifier {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::card::CastingModifier) -> Result<Self, Self::Error> {
        value
            .modifier
            .as_ref()
            .ok_or_else(|| anyhow!("Expected modifier to have a modifier specified"))
            .map(Self::from)
    }
}

impl From<&protogen::card::casting_modifier::Modifier> for CastingModifier {
    fn from(value: &protogen::card::casting_modifier::Modifier) -> Self {
        match value {
            protogen::card::casting_modifier::Modifier::CannotBeCountered(_) => {
                Self::CannotBeCountered
            }
            protogen::card::casting_modifier::Modifier::SplitSecond(_) => Self::SplitSecond,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct Card {
    pub name: String,
    pub types: EnumSet<Type>,
    pub subtypes: EnumSet<Subtype>,

    pub cost: CastingCost,
    pub casting_modifiers: EnumSet<CastingModifier>,
    pub colors: EnumSet<Color>,

    pub oracle_text: String,
    pub flavor_text: String,

    pub etb_abilities: Vec<ETBAbility>,
    pub effects: Vec<SpellEffect>,

    pub static_abilities: Vec<StaticAbility>,
    pub activated_abilities: Vec<ActivatedAbility>,

    pub power: Option<usize>,
    pub toughness: Option<usize>,

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
                .collect::<anyhow::Result<_>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<_>>()?,
            cost: value
                .cost
                .as_ref()
                .ok_or_else(|| anyhow!("Expected a casting cost"))?
                .try_into()?,
            casting_modifiers: value
                .casting_modifiers
                .iter()
                .map(CastingModifier::try_from)
                .collect::<anyhow::Result<_>>()?,
            colors: value
                .colors
                .iter()
                .map(Color::try_from)
                .collect::<anyhow::Result<_>>()?,
            oracle_text: value.oracle_text,
            flavor_text: value.flavor_text,
            etb_abilities: value
                .etb_abilities
                .iter()
                .map(ETBAbility::try_from)
                .collect::<anyhow::Result<_>>()?,
            effects: value
                .effects
                .iter()
                .map(SpellEffect::try_from)
                .collect::<anyhow::Result<_>>()?,
            static_abilities: value
                .static_abilities
                .iter()
                .map(StaticAbility::try_from)
                .collect::<anyhow::Result<_>>()?,
            activated_abilities: value
                .activated_abilities
                .iter()
                .map(ActivatedAbility::try_from)
                .collect::<anyhow::Result<_>>()?,
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
            hexproof: value.hexproof,
            shroud: value.shroud,
        })
    }
}

impl Card {
    pub fn color(&self) -> EnumSet<Color> {
        let mut colors = EnumSet::default();
        for mana in self.cost.mana_cost.iter() {
            let color = mana.color();
            colors.insert(color);
        }
        for color in self.colors.iter() {
            colors.insert(color);
        }

        colors
    }

    pub fn color_identity(&self) -> EnumSet<Color> {
        let mut identity = self.color();

        for ability in self.activated_abilities.iter() {
            for mana in ability.cost.mana_cost.iter() {
                let color = mana.color();
                identity.insert(color);
            }
        }

        identity
    }

    pub fn uses_stack(&self) -> bool {
        !self.is_land()
    }

    pub fn is_land(&self) -> bool {
        self.types
            .iter()
            .any(|ty| matches!(ty, Type::BasicLand | Type::Land))
    }

    pub fn is_permanent(&self) -> bool {
        self.types.iter().any(|ty| ty.is_permanent())
    }

    pub fn requires_target(&self) -> bool {
        for effect in self.effects.iter() {
            match effect {
                SpellEffect::CounterSpell { .. } => return true,
                SpellEffect::GainMana { .. } => {}
                SpellEffect::BattlefieldModifier(_) => {}
                SpellEffect::ControllerDrawCards(_) => {}
                SpellEffect::AddPowerToughnessToTarget(_) => return true,
                SpellEffect::ModifyCreature(_) => return true,
                SpellEffect::ExileTargetCreature => return true,
                SpellEffect::ExileTargetCreatureManifestTopOfLibrary => return false,
            }
        }
        false
    }
}
