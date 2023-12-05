use anyhow::anyhow;
use bevy_ecs::{bundle::Bundle, component::Component, entity::Entity, system::Query};
use derive_more::{Deref, DerefMut, From};
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
        base: &CardTypes,
        query: &Query<&ModifyingTypeSet>,
    ) -> anyhow::Result<EnumSet<Type>> {
        let mut types = **base;
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
        base: &CardSubtypes,
        query: &Query<&ModifyingSubtypeSet>,
    ) -> anyhow::Result<EnumSet<Subtype>> {
        let mut types = **base;
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

#[derive(Debug, Clone, Copy, Component)]
pub enum PowerModifier {
    SetBase(i32),
    Add(i32),
}

#[derive(Debug, Component)]
pub struct ModifyingPower(IndexSet<Entity>);

impl ModifyingPower {
    pub fn power(
        &self,
        power: &Power,
        query: &Query<&PowerModifier>,
    ) -> anyhow::Result<Option<i32>> {
        let mut base = **power;
        let mut add = 0;
        for modifier in self.0.iter().copied() {
            match query.get(modifier)? {
                PowerModifier::SetBase(new_base) => {
                    base = Some(*new_base);
                }
                PowerModifier::Add(also_add) => {
                    add += also_add;
                }
            }
        }

        Ok(base.map(|base| base + add))
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub enum ToughnessModifier {
    SetBase(i32),
    Add(i32),
}

#[derive(Debug, Component)]
pub struct ModifyingToughness(IndexSet<Entity>);

impl ModifyingToughness {
    pub fn toughness(
        &self,
        toughness: &Toughness,
        modifiers: &Query<&ToughnessModifier>,
    ) -> anyhow::Result<Option<i32>> {
        let mut base = **toughness;
        let mut add = 0;
        for modifier in self.0.iter().copied() {
            match modifiers.get(modifier)? {
                ToughnessModifier::SetBase(new_base) => {
                    base = Some(*new_base);
                }
                ToughnessModifier::Add(also_add) => {
                    add += also_add;
                }
            }
        }

        Ok(base.map(|base| base + add))
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

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct CardName(String);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct CardTypes(EnumSet<Type>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct CardSubtypes(EnumSet<Subtype>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct OracleText(String);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct FlavorText(String);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct CastingModifiers(EnumSet<CastingModifier>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct Colors(EnumSet<Color>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct ETBAbilities(Vec<ETBAbility>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct SpellEffects(Vec<SpellEffect>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct StaticAbilities(Vec<StaticAbility>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct ActivatedAbilities(Vec<ActivatedAbility>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct Power(Option<i32>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct Toughness(Option<i32>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct Hexproof(bool);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, From, Component)]
pub struct Shroud(bool);

#[derive(Debug, Clone, PartialEq, Eq, Bundle)]
pub struct Card {
    pub name: CardName,
    pub types: CardTypes,
    pub subtypes: CardSubtypes,

    pub cost: CastingCost,
    pub casting_modifiers: CastingModifiers,
    pub colors: Colors,

    pub oracle_text: OracleText,
    pub flavor_text: FlavorText,

    pub etb_abilities: ETBAbilities,
    pub effects: SpellEffects,

    pub static_abilities: StaticAbilities,
    pub activated_abilities: ActivatedAbilities,

    pub power: Power,
    pub toughness: Toughness,

    pub hexproof: Hexproof,
    pub shroud: Shroud,
}

impl TryFrom<protogen::card::Card> for Card {
    type Error = anyhow::Error;

    fn try_from(value: protogen::card::Card) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.into(),
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?
                .into(),
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?
                .into(),
            cost: value
                .cost
                .as_ref()
                .ok_or_else(|| anyhow!("Expected a casting cost"))?
                .try_into()?,
            casting_modifiers: value
                .casting_modifiers
                .iter()
                .map(CastingModifier::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?
                .into(),
            colors: value
                .colors
                .iter()
                .map(Color::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?
                .into(),
            oracle_text: value.oracle_text.into(),
            flavor_text: value.flavor_text.into(),
            etb_abilities: value
                .etb_abilities
                .iter()
                .map(ETBAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?
                .into(),
            effects: value
                .effects
                .iter()
                .map(SpellEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?
                .into(),
            static_abilities: value
                .static_abilities
                .iter()
                .map(StaticAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?
                .into(),
            activated_abilities: value
                .activated_abilities
                .iter()
                .map(ActivatedAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?
                .into(),
            power: value.power.into(),
            toughness: value.toughness.into(),
            hexproof: value.hexproof.into(),
            shroud: value.shroud.into(),
        })
    }
}

impl Card {
    pub fn colors(cost: &CastingCost, colors: &Colors) -> EnumSet<Color> {
        let mut derived_colors = EnumSet::default();
        for mana in cost.mana_cost.iter() {
            let color = mana.color();
            derived_colors.insert(color);
        }
        for color in colors.iter() {
            derived_colors.insert(color);
        }

        derived_colors
    }

    pub fn color_identity(
        cost: &CastingCost,
        colors: &Colors,
        activated_abilities: &ActivatedAbilities,
    ) -> EnumSet<Color> {
        let mut identity = Self::colors(cost, colors);

        for ability in activated_abilities.iter() {
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

    pub fn is_permanent(types: &CardTypes) -> bool {
        types.iter().any(|ty| ty.is_permanent())
    }

    pub fn requires_target(effects: &SpellEffects) -> bool {
        for effect in effects.iter() {
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
