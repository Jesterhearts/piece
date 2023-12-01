use std::collections::HashSet;

use anyhow::anyhow;

use crate::{
    abilities::{ActivatedAbility, StaticAbility},
    battlefield::Battlefield,
    controller::Controller,
    cost::CastingCost,
    effects::Effect,
    in_play::AllCards,
    player::Player,
    protogen,
    stack::{ActiveTarget, EntryType, Stack},
    targets::{SpellTarget, Target},
    types::{Subtype, Type},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CastingModifier {
    CannotBeCountered,
    SplitSecond,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Card {
    pub name: String,
    pub ty: Type,
    pub subtypes: Vec<Subtype>,

    pub cost: CastingCost,
    pub casting_modifiers: Vec<CastingModifier>,
    pub colors: Vec<Color>,

    pub oracle_text: String,
    pub flavor_text: String,

    pub effects: Vec<Effect>,
    pub static_abilities: Vec<StaticAbility>,
    pub activated_abilities: Vec<ActivatedAbility>,

    pub targets: Vec<Target>,

    pub power: Option<usize>,
    pub toughness: Option<usize>,
}

impl TryFrom<protogen::card::Card> for Card {
    type Error = anyhow::Error;

    fn try_from(value: protogen::card::Card) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            ty: value
                .ty
                .ty
                .as_ref()
                .ok_or_else(|| anyhow!("Expected card to have a type"))?
                .into(),
            subtypes: value
                .subtypes
                .iter()
                .map(|subtype| {
                    subtype
                        .subtype
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected subtype to have a subtype specified"))
                        .map(Subtype::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            cost: value
                .cost
                .as_ref()
                .ok_or_else(|| anyhow!("Expected a casting cost"))?
                .try_into()?,
            casting_modifiers: value
                .casting_modifiers
                .iter()
                .map(|modifier| {
                    modifier
                        .modifier
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected modifier to have a modifier specified"))
                        .map(CastingModifier::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            colors: value
                .colors
                .iter()
                .map(|color| {
                    color
                        .color
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected color to have a color set"))
                        .map(Color::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            oracle_text: value.oracle_text,
            flavor_text: value.flavor_text,
            effects: value
                .effects
                .iter()
                .map(|effect| {
                    effect
                        .effect
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected effect to have an effect specified"))
                        .and_then(Effect::try_from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            static_abilities: value
                .static_abilities
                .iter()
                .map(|ability| {
                    ability
                        .ability
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected ability to have an ability specified"))
                        .and_then(StaticAbility::try_from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            activated_abilities: value
                .activated_abilities
                .iter()
                .map(ActivatedAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            targets: value
                .targets
                .iter()
                .map(|target| {
                    target
                        .target
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected target to have a target specified"))
                        .and_then(Target::try_from)
                })
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
        })
    }
}

impl Card {
    pub fn color(&self) -> HashSet<Color> {
        let mut colors = HashSet::default();
        for mana in self.cost.mana_cost.iter() {
            let color = mana.color();
            colors.insert(color);
        }
        for color in self.colors.iter() {
            colors.insert(*color);
        }

        colors
    }

    pub fn color_identity(&self) -> HashSet<Color> {
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

    pub fn requires_target(&self) -> bool {
        !self.targets.is_empty()
    }

    pub fn is_land(&self) -> bool {
        matches!(self.ty, Type::BasicLand | Type::Land)
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
                Target::Spell(SpellTarget {
                    controller: _,
                    types: _,
                    subtypes: _,
                }) => {
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
                Target::Creature { subtypes: _ } => todo!(),
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
        target: &SpellTarget,
    ) -> bool {
        for modifier in self.casting_modifiers.iter() {
            match modifier {
                CastingModifier::CannotBeCountered => {
                    return false;
                }
                _ => {}
            }
        }

        let SpellTarget {
            controller,
            types,
            subtypes,
        } = target;
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
