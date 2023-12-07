use anyhow::anyhow;
use enumset::{EnumSet, EnumSetType};
use indexmap::IndexMap;

use crate::{
    abilities::{ActivatedAbility, ETBAbility, Enchant, StaticAbility, TriggeredAbility},
    battlefield::Battlefield,
    controller::Controller,
    cost::CastingCost,
    effects::{
        ActivatedAbilityEffect, AddCreatureSubtypes, AddPowerToughness, BattlefieldModifier,
        GainMana, ModifyBasePowerToughness, ModifyBattlefield, SpellEffect, Token, TokenCreature,
    },
    in_play::{AllCards, CardId, ModifierId},
    player::Player,
    protogen,
    stack::{ActiveTarget, EntryType, Stack},
    targets::SpellTarget,
    types::{Subtype, Type},
};

#[derive(Debug, EnumSetType)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeModifier {
    RemoveAll,
    Add(EnumSet<Type>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtypeModifier {
    RemoveAll,
    Add(EnumSet<Subtype>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticAbilityModifier {
    RemoveAll,
    Add(StaticAbility),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivatedAbilityModifier {
    RemoveAll,
    Add(ActivatedAbility),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Card {
    pub name: String,
    types: EnumSet<Type>,
    subtypes: EnumSet<Subtype>,

    pub modified_types: IndexMap<ModifierId, TypeModifier>,
    pub modified_subtypes: IndexMap<ModifierId, SubtypeModifier>,

    pub cost: CastingCost,
    pub casting_modifiers: Vec<CastingModifier>,
    pub colors: EnumSet<Color>,

    pub oracle_text: String,
    pub flavor_text: String,

    pub enchant: Option<Enchant>,

    pub etb_abilities: Vec<ETBAbility>,
    pub effects: Vec<SpellEffect>,

    static_abilities: Vec<StaticAbility>,
    pub adjusted_static_abilities: IndexMap<ModifierId, StaticAbilityModifier>,

    activated_abilities: Vec<ActivatedAbility>,
    pub adjusted_activated_abilities: IndexMap<ModifierId, ActivatedAbilityModifier>,

    pub triggered_abilities: Vec<TriggeredAbility>,

    power: Option<usize>,
    toughness: Option<usize>,

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
                .map(Type::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
            modified_types: Default::default(),
            modified_subtypes: Default::default(),
            cost: value
                .cost
                .as_ref()
                .ok_or_else(|| anyhow!("Expected a casting cost"))?
                .try_into()?,
            casting_modifiers: value
                .casting_modifiers
                .iter()
                .map(CastingModifier::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            colors: value
                .colors
                .iter()
                .map(Color::try_from)
                .collect::<anyhow::Result<EnumSet<_>>>()?,
            oracle_text: value.oracle_text,
            flavor_text: value.flavor_text,
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
                .map(SpellEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            static_abilities: value
                .static_abilities
                .iter()
                .map(StaticAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            adjusted_static_abilities: Default::default(),
            activated_abilities: value
                .activated_abilities
                .iter()
                .map(ActivatedAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            adjusted_activated_abilities: Default::default(),
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
            adjusted_base_power: Default::default(),
            adjusted_base_toughness: Default::default(),
            power_modifier: Default::default(),
            toughness_modifier: Default::default(),
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

    pub fn types(&self) -> EnumSet<Type> {
        let mut types = self.types;

        for modified_types in self.modified_types.values() {
            match *modified_types {
                TypeModifier::RemoveAll => types.clear(),
                TypeModifier::Add(tys) => types.extend(tys),
            }
        }

        types
    }

    pub fn subtypes(&self) -> EnumSet<Subtype> {
        let mut subtypes = self.subtypes;

        for modified_subtypes in self.modified_subtypes.values() {
            match *modified_subtypes {
                SubtypeModifier::RemoveAll => subtypes.clear(),
                SubtypeModifier::Add(types) => subtypes.extend(types),
            }
        }

        subtypes
    }

    pub fn power(&self) -> Option<i32> {
        let base = self
            .adjusted_base_power
            .last()
            .map(|(_, v)| *v)
            .or(self.power.map(|p| p as i32));
        let modifier: i32 = self.power_modifier.iter().map(|(_, v)| *v).sum();

        base.map(|base| base + modifier)
    }

    pub fn toughness(&self) -> Option<i32> {
        let base = self
            .adjusted_base_toughness
            .last()
            .map(|(_, v)| *v)
            .or(self.toughness.map(|p| p as i32));
        let modifier: i32 = self.toughness_modifier.iter().map(|(_, v)| *v).sum();

        base.map(|base| base + modifier)
    }

    pub fn color_identity(&self) -> EnumSet<Color> {
        let mut identity = self.color();

        for ability in self.activated_abilities.iter() {
            for mana in ability.cost.mana_cost.iter() {
                let color = mana.color();
                identity.insert(color);
            }

            for effect in ability.effects.iter() {
                match effect {
                    ActivatedAbilityEffect::GainMana { mana } => match mana {
                        GainMana::Specific { gains } => {
                            for gain in gains.iter() {
                                identity.insert(gain.color());
                            }
                        }
                        GainMana::Choice { choices } => {
                            for choice in choices.iter() {
                                for mana in choice.iter() {
                                    identity.insert(mana.color());
                                }
                            }
                        }
                    },
                    _ => {}
                }
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
    ) -> Vec<ActiveTarget> {
        let mut targets = Vec::default();
        let creatures = battlefield.creatures(cards);

        for effect in self.effects.iter() {
            match effect {
                SpellEffect::CounterSpell { target } => {
                    targets_for_counterspell(
                        stack,
                        cards,
                        battlefield,
                        caster,
                        target,
                        &mut targets,
                    );
                }
                SpellEffect::GainMana { .. } => {}
                SpellEffect::BattlefieldModifier(_) => {}
                SpellEffect::ControllerDrawCards(_) => {}
                SpellEffect::AddPowerToughnessToTarget(_) => {
                    for creature in battlefield.creatures(cards) {
                        let card = &cards[creature];
                        if card.card.can_be_targeted(caster, &card.controller.borrow()) {
                            targets.push(ActiveTarget::Battlefield { id: creature });
                        }
                    }
                }
                SpellEffect::ModifyCreature(modifier) => {
                    targets_for_battlefield_modifier(
                        &modifier.modifier,
                        &creatures,
                        cards,
                        caster,
                        &mut targets,
                    );
                }
                SpellEffect::ExileTargetCreature => {
                    for creature in creatures.iter() {
                        let card = &cards[*creature];
                        if card.card.can_be_targeted(caster, &card.controller.borrow()) {
                            targets.push(ActiveTarget::Battlefield { id: *creature });
                        }
                    }
                }
                SpellEffect::ExileTargetCreatureManifestTopOfLibrary => {
                    for creature in creatures.iter() {
                        let card = &cards[*creature];
                        if card.card.can_be_targeted(caster, &card.controller.borrow()) {
                            targets.push(ActiveTarget::Battlefield { id: *creature });
                        }
                    }
                }
            }
        }

        for ability in self.activated_abilities() {
            for effect in ability.effects {
                match effect {
                    ActivatedAbilityEffect::CounterSpell { target } => {
                        targets_for_counterspell(
                            stack,
                            cards,
                            battlefield,
                            caster,
                            &target,
                            &mut targets,
                        );
                    }
                    ActivatedAbilityEffect::GainMana { .. } => {}
                    ActivatedAbilityEffect::BattlefieldModifier(_) => {}
                    ActivatedAbilityEffect::ControllerDrawCards(_) => {}
                    ActivatedAbilityEffect::Equip(equip) => {
                        for modifier in equip {
                            targets_for_battlefield_modifier(
                                &modifier,
                                &creatures,
                                cards,
                                caster,
                                &mut targets,
                            );
                        }
                    }
                    ActivatedAbilityEffect::AddPowerToughnessToTarget(_) => {
                        for creature in battlefield.creatures(cards) {
                            let card = &cards[creature];
                            if card.card.can_be_targeted(caster, &card.controller.borrow()) {
                                targets.push(ActiveTarget::Battlefield { id: creature });
                            }
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

        if !types.is_empty() && !self.types_intersect(*types) {
            return false;
        }

        if !self.subtypes_match(*subtypes) {
            return false;
        }

        for (ability, ability_controller) in battlefield.static_abilities(cards).into_iter() {
            match &ability {
                StaticAbility::GreenCannotBeCountered { controller } => {
                    if self.color().contains(Color::Green) {
                        match controller {
                            Controller::You => {
                                if ability_controller == *this_controller {
                                    return false;
                                }
                            }
                            Controller::Opponent => {
                                if ability_controller != *this_controller {
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
                StaticAbility::Flash => {}
                StaticAbility::BattlefieldModifier(_) => {}
            }
        }

        true
    }

    pub fn can_be_sacrificed(&self, _battlefield: &Battlefield) -> bool {
        // TODO: check battlefield for effects preventing sacrifice
        true
    }

    pub fn types_intersect(&self, types: EnumSet<Type>) -> bool {
        let self_types = self.types();
        types.is_empty() || !self_types.is_disjoint(types)
    }

    pub fn subtypes_intersect(&self, subtypes: EnumSet<Subtype>) -> bool {
        let self_subtypes = self.subtypes();
        subtypes.is_empty() || !self_subtypes.is_disjoint(subtypes)
    }

    pub fn subtypes_match(&self, subtypes: EnumSet<Subtype>) -> bool {
        let self_subtypes = self.subtypes();
        subtypes.is_subset(self_subtypes)
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
                SpellEffect::AddPowerToughnessToTarget(_) => return true,
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
            ModifyBattlefield::RemoveAllSubtypes(_) => {
                self.modified_subtypes.remove(&id);
            }
            ModifyBattlefield::Vigilance => {
                self.adjusted_static_abilities.remove(&id);
            }
            ModifyBattlefield::RemoveAllAbilities => {
                self.adjusted_static_abilities.remove(&id);
                self.adjusted_activated_abilities.remove(&id);
            }
        }
    }

    pub(crate) fn add_modifier(&mut self, id: ModifierId, modifier: &BattlefieldModifier) {
        match &modifier.modifier {
            ModifyBattlefield::ModifyBasePowerToughness(ModifyBasePowerToughness {
                power,
                toughness,
            }) => {
                self.adjusted_base_power.insert(id, *power);
                self.adjusted_base_toughness.insert(id, *toughness);
            }
            ModifyBattlefield::AddCreatureSubtypes(AddCreatureSubtypes {
                add_types,
                add_subtypes,
            }) => {
                self.modified_subtypes
                    .insert(id, SubtypeModifier::Add(*add_subtypes));
                self.modified_types
                    .insert(id, TypeModifier::Add(*add_types));
            }
            ModifyBattlefield::AddPowerToughness(AddPowerToughness { power, toughness }) => {
                self.power_modifier.insert(id, *power);
                self.toughness_modifier.insert(id, *toughness);
            }
            ModifyBattlefield::RemoveAllSubtypes(_) => {
                self.modified_subtypes
                    .insert(id, SubtypeModifier::RemoveAll);
            }
            ModifyBattlefield::Vigilance => {
                self.adjusted_static_abilities
                    .insert(id, StaticAbilityModifier::Add(StaticAbility::Vigilance));
            }
            ModifyBattlefield::RemoveAllAbilities => {
                self.adjusted_static_abilities
                    .insert(id, StaticAbilityModifier::RemoveAll);
                self.adjusted_activated_abilities
                    .insert(id, ActivatedAbilityModifier::RemoveAll);
            }
        }
    }

    pub(crate) fn static_abilities(&self) -> Vec<StaticAbility> {
        let mut abilities = self.static_abilities.clone();
        for modifier in self.adjusted_static_abilities.values() {
            match modifier {
                StaticAbilityModifier::RemoveAll => abilities.clear(),
                StaticAbilityModifier::Add(ability) => abilities.push(ability.clone()),
            }
        }

        abilities
    }

    pub(crate) fn activated_abilities(&self) -> Vec<ActivatedAbility> {
        let mut abilities = self.activated_abilities.clone();
        for modifier in self.adjusted_activated_abilities.values() {
            match modifier {
                ActivatedAbilityModifier::RemoveAll => abilities.clear(),
                ActivatedAbilityModifier::Add(ability) => abilities.push(ability.clone()),
            }
        }

        abilities
    }
}

fn targets_for_battlefield_modifier(
    modifier: &ModifyBattlefield,
    creatures: &[CardId],
    cards: &AllCards,
    caster: &Player,
    targets: &mut Vec<ActiveTarget>,
) {
    match &modifier {
        ModifyBattlefield::ModifyBasePowerToughness(ModifyBasePowerToughness { .. }) => {
            for creature in creatures.iter() {
                let card = &cards[*creature];
                if card.card.can_be_targeted(caster, &card.controller.borrow()) {
                    targets.push(ActiveTarget::Battlefield { id: *creature });
                }
            }
        }
        ModifyBattlefield::AddCreatureSubtypes(_)
        | ModifyBattlefield::AddPowerToughness(_)
        | ModifyBattlefield::RemoveAllSubtypes(_)
        | ModifyBattlefield::Vigilance
        | ModifyBattlefield::RemoveAllAbilities => {
            for creature in creatures.iter() {
                let card = &cards[*creature];
                if card.card.can_be_targeted(caster, &card.controller.borrow()) {
                    targets.push(ActiveTarget::Battlefield { id: *creature });
                }
            }
        }
    };
}

fn targets_for_counterspell(
    stack: &Stack,
    cards: &AllCards,
    battlefield: &Battlefield,
    caster: &Player,
    target: &SpellTarget,
    targets: &mut Vec<ActiveTarget>,
) {
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
                    targets.push(ActiveTarget::Stack { id: *index });
                }
            }
            EntryType::ActivatedAbility { .. } => {}
        }
    }
}
