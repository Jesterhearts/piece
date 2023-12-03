use std::collections::HashSet;

use anyhow::anyhow;
use indexmap::IndexMap;

use crate::{
    abilities::{ActivatedAbility, ETBAbility, StaticAbility},
    battlefield::Battlefield,
    controller::Controller,
    cost::CastingCost,
    effects::{
        AddCreatureSubtypes, AddPowerToughness, BattlefieldModifier, ModifyBasePowerToughness,
        ModifyBattlefield, SpellEffect,
    },
    in_play::{AllCards, ModifierId},
    player::Player,
    protogen,
    stack::{ActiveTarget, EntryType, Stack},
    targets::SpellTarget,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub name: String,
    pub types: HashSet<Type>,
    pub subtypes: HashSet<Subtype>,

    pub modified_subtypes: IndexMap<ModifierId, HashSet<Subtype>>,
    pub remove_all_subtypes: HashSet<ModifierId>,

    pub cost: CastingCost,
    pub casting_modifiers: Vec<CastingModifier>,
    pub colors: Vec<Color>,

    pub oracle_text: String,
    pub flavor_text: String,

    pub etb_abilities: Vec<ETBAbility>,
    pub effects: Vec<SpellEffect>,
    pub static_abilities: HashSet<StaticAbility>,
    pub activated_abilities: Vec<ActivatedAbility>,

    pub power: Option<usize>,
    pub toughness: Option<usize>,

    pub adjusted_base_power: IndexMap<ModifierId, i32>,
    pub adjusted_base_toughness: IndexMap<ModifierId, i32>,

    pub power_modifier: IndexMap<ModifierId, i32>,
    pub toughness_modifier: IndexMap<ModifierId, i32>,

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
                .map(|ty| {
                    ty.ty
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected type to have a type specified"))
                        .map(Type::from)
                })
                .collect::<anyhow::Result<HashSet<_>>>()?,
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
                .collect::<anyhow::Result<HashSet<_>>>()?,
            modified_subtypes: Default::default(),
            remove_all_subtypes: Default::default(),
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
            etb_abilities: value
                .etb_abilities
                .iter()
                .map(|ability| {
                    ability
                        .ability
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected etb ability to have an ability specified"))
                        .map(ETBAbility::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            effects: value
                .effects
                .iter()
                .map(|effect| {
                    effect
                        .effect
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected effect to have an effect specified"))
                        .and_then(SpellEffect::try_from)
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
                .collect::<anyhow::Result<HashSet<_>>>()?,
            activated_abilities: value
                .activated_abilities
                .iter()
                .map(ActivatedAbility::try_from)
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
            adjusted_base_power: Default::default(),
            adjusted_base_toughness: Default::default(),
            power_modifier: Default::default(),
            toughness_modifier: Default::default(),
            hexproof: value.hexproof,
            shroud: value.shroud,
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

    pub fn subtypes(&self) -> HashSet<Subtype> {
        if !self.remove_all_subtypes.is_empty() {
            return Default::default();
        }

        let mut subtypes = self.subtypes.clone();

        for modified_subtypes in self.modified_subtypes.values() {
            subtypes.extend(modified_subtypes.iter());
        }

        subtypes
    }

    pub fn power(&self) -> i32 {
        let base = self
            .adjusted_base_power
            .last()
            .map(|(_, v)| *v)
            .or(self.power.map(|p| p as i32))
            .unwrap_or_default();
        let modifier: i32 = self.power_modifier.iter().map(|(_, v)| *v).sum();

        base + modifier
    }

    pub fn toughness(&self) -> i32 {
        let base = self
            .adjusted_base_toughness
            .last()
            .map(|(_, v)| *v)
            .or(self.toughness.map(|p| p as i32))
            .unwrap_or_default();
        let modifier: i32 = self.toughness_modifier.iter().map(|(_, v)| *v).sum();

        base + modifier
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

    pub fn is_land(&self) -> bool {
        self.types
            .iter()
            .any(|ty| matches!(ty, Type::BasicLand | Type::Land))
    }

    pub fn valid_targets(
        &self,
        cards: &AllCards,
        battlefield: &Battlefield,
        stack: &Stack,
        caster: &Player,
    ) -> HashSet<ActiveTarget> {
        let mut targets = HashSet::default();
        let creatures = battlefield.creatures(cards);

        for effect in self.effects.iter() {
            match effect {
                SpellEffect::CounterSpell { target } => {
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
                            EntryType::ActivatedAbility { .. } => {}
                        }
                    }
                }
                SpellEffect::GainMana { .. } => {}
                SpellEffect::BattlefieldModifier(_) => {}
                SpellEffect::ControllerDrawCards(_) => {}
                SpellEffect::AddPowerToughness(_) => {
                    for creature in battlefield.creatures(cards) {
                        targets.insert(ActiveTarget::Battlefield { id: creature });
                    }
                }
                SpellEffect::ModifyCreature(modifier) => match &modifier.modifier {
                    ModifyBattlefield::ModifyBasePowerToughness(ModifyBasePowerToughness {
                        targets: target_types,
                        ..
                    }) => {
                        for creature in creatures.iter() {
                            let card = &cards[*creature];
                            if card.card.subtypes_intersect(target_types)
                                && card.card.can_be_targeted(caster, &card.controller.borrow())
                            {
                                targets.insert(ActiveTarget::Battlefield { id: *creature });
                            }
                        }
                    }
                    ModifyBattlefield::AddCreatureSubtypes(_)
                    | ModifyBattlefield::AddPowerToughness(_)
                    | ModifyBattlefield::RemoveAllSubtypes => {
                        for creature in creatures.iter() {
                            let card = &cards[*creature];
                            if card.card.can_be_targeted(caster, &card.controller.borrow()) {
                                targets.insert(ActiveTarget::Battlefield { id: *creature });
                            }
                        }
                    }
                },
                SpellEffect::ExileTargetCreature => {
                    for creature in creatures.iter() {
                        let card = &cards[*creature];
                        if card.card.can_be_targeted(caster, &card.controller.borrow()) {
                            targets.insert(ActiveTarget::Battlefield { id: *creature });
                        }
                    }
                }
                SpellEffect::ExileTargetCreatureManifestTopOfLibrary => {
                    for creature in creatures.iter() {
                        let card = &cards[*creature];
                        if card.card.can_be_targeted(caster, &card.controller.borrow()) {
                            targets.insert(ActiveTarget::Battlefield { id: *creature });
                        }
                    }
                }
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
                StaticAbility::Vigilance => {}
                StaticAbility::BattlefieldModifier(_) => {}
            }
        }

        true
    }

    pub fn can_be_sacrificed(&self, _battlefield: &Battlefield) -> bool {
        // TODO: check battlefield for effects preventing sacrifice
        true
    }

    pub fn types_intersect(&self, types: &[Type]) -> bool {
        types.iter().any(|ty| self.types.contains(ty))
    }

    pub fn subtypes_intersect(&self, subtypes: &[Subtype]) -> bool {
        let self_subtypes = self.subtypes();
        subtypes.is_empty() || subtypes.iter().any(|ty| self_subtypes.contains(ty))
    }

    pub fn subtypes_match(&self, subtypes: &[Subtype]) -> bool {
        let self_subtypes = self.subtypes();
        subtypes.iter().all(|ty| self_subtypes.contains(ty))
    }

    pub fn is_permanent(&self) -> bool {
        self.types.iter().any(|ty| ty.is_permanent())
    }

    pub(crate) fn can_be_targeted(
        &self,
        source_controller: &Player,
        this_controller: &Player,
    ) -> bool {
        if self.shroud {
            return false;
        }

        if self.hexproof && *source_controller != *this_controller {
            return false;
        }

        // TODO protection

        true
    }

    pub(crate) fn requires_target(&self) -> bool {
        for effect in self.effects.iter() {
            match effect {
                SpellEffect::CounterSpell { .. } => return true,
                SpellEffect::GainMana { .. } => {}
                SpellEffect::BattlefieldModifier(_) => {}
                SpellEffect::ControllerDrawCards(_) => {}
                SpellEffect::AddPowerToughness(_) => return true,
                SpellEffect::ModifyCreature(_) => return true,
                SpellEffect::ExileTargetCreature => return true,
                SpellEffect::ExileTargetCreatureManifestTopOfLibrary => return false,
            }
        }
        false
    }

    pub(crate) fn remove_modifier(&mut self, id: ModifierId, modifier: &BattlefieldModifier) {
        match &modifier.modifier {
            ModifyBattlefield::ModifyBasePowerToughness(_) => {
                self.adjusted_base_power.remove(&id);
                self.adjusted_base_toughness.remove(&id);
            }
            ModifyBattlefield::AddCreatureSubtypes(_) => {
                self.modified_subtypes.remove(&id);
            }
            ModifyBattlefield::AddPowerToughness(_) => {
                self.power_modifier.remove(&id);
                self.toughness_modifier.remove(&id);
            }
            ModifyBattlefield::RemoveAllSubtypes => {
                self.remove_all_subtypes.remove(&id);
            }
        }
    }

    pub(crate) fn add_modifier(&mut self, id: ModifierId, modifier: &BattlefieldModifier) {
        match &modifier.modifier {
            ModifyBattlefield::ModifyBasePowerToughness(ModifyBasePowerToughness {
                targets,
                power,
                toughness,
            }) => {
                if self.subtypes_intersect(targets) {
                    self.adjusted_base_power.insert(id, *power);
                    self.adjusted_base_toughness.insert(id, *toughness);
                }
            }
            ModifyBattlefield::AddCreatureSubtypes(AddCreatureSubtypes { targets, types }) => {
                if self.subtypes_intersect(targets) {
                    self.modified_subtypes
                        .insert(id, types.iter().copied().collect());
                }
            }
            ModifyBattlefield::AddPowerToughness(AddPowerToughness { power, toughness }) => {
                self.power_modifier.insert(id, *power);
                self.toughness_modifier.insert(id, *toughness);
            }
            ModifyBattlefield::RemoveAllSubtypes => {
                self.remove_all_subtypes.insert(id);
            }
        }
    }
}
