use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, EventWriter, Events},
    query::{With, Without},
    system::{Commands, Query, ResMut, Resource},
};
use indexmap::{IndexMap, IndexSet};

use crate::{
    abilities::StaticAbility,
    activated_ability::ActiveAbility,
    battlefield::{Battlefield, BattlefieldId, EtbEvent, GraveyardId},
    card::{
        Card, CardSubtypes, CardTypes, CastingModifier, CastingModifiers, Color, Colors,
        ModifyingPower, ModifyingSubtypeSet, ModifyingSubtypes, ModifyingToughness,
        ModifyingTypeSet, ModifyingTypes, PowerModifier, SpellEffects, StaticAbilities,
        ToughnessModifier,
    },
    controller::Controller,
    cost::CastingCost,
    effects::{ActivatedAbilityEffect, ModifyBattlefield, SpellEffect},
    player::{self, Owner},
    targets::{Restriction, SpellTarget},
    types::Type,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct StackId(usize);

#[derive(Debug, Clone, Component)]
pub enum Targets {
    Stack(Vec<StackId>),
    Battlefield(Vec<BattlefieldId>),
    Graveyard(Vec<GraveyardId>),
    Player(Vec<Owner>),
    Entities(Vec<Entity>),
}

impl Targets {
    pub fn len(&self) -> usize {
        match self {
            Targets::Stack(targets) => targets.len(),
            Targets::Battlefield(targets) => targets.len(),
            Targets::Graveyard(targets) => targets.len(),
            Targets::Player(targets) => targets.len(),
            Targets::Entities(targets) => targets.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StackEntry {
    Spell(Entity),
    ActivatedAbility(Entity),
    TriggeredAbility(Entity),
}

impl StackEntry {
    fn entity(&self) -> Entity {
        match self {
            StackEntry::Spell(e)
            | StackEntry::ActivatedAbility(e)
            | StackEntry::TriggeredAbility(e) => *e,
        }
    }
}

#[derive(Debug, Event)]
pub enum StackResult {
    StackToGraveyard(Entity),
    StackToBattlefield(Entity),
}

#[derive(Debug, Event)]
pub struct AddToStackEvent {
    pub entry: StackEntry,
    pub target: Option<Targets>,
}

#[derive(Debug, Default, Resource)]
pub struct Stack {
    entries: IndexMap<StackId, StackEntry>,
    next_id: usize,
    pub split_second: bool,
}

impl Stack {
    fn next_id(&mut self) -> StackId {
        let id = self.next_id;
        self.next_id += 1;
        StackId(id)
    }

    pub fn target_nth(&self, nth: usize) -> Option<StackId> {
        self.entries.get_index(nth).map(|(id, _)| *id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn add_to_stack(
    mut stack: ResMut<Stack>,
    mut commands: Commands,
    mut queue: ResMut<Events<AddToStackEvent>>,
    cards: Query<&CastingModifiers>,
) {
    for entry in queue.drain() {
        let entity = entry.entry.entity();
        let id = stack.next_id();
        stack.entries.insert(id, entry.entry);
        commands.entity(entity).insert(id);
        if let Some(target) = entry.target {
            commands.entity(entity).insert(target);
        }

        match entry.entry {
            StackEntry::Spell(entity) => {
                if cards
                    .get(entity)
                    .unwrap()
                    .contains(CastingModifier::SplitSecond)
                {
                    stack.split_second = true;
                    break;
                }
            }
            StackEntry::ActivatedAbility(_) => {}
            StackEntry::TriggeredAbility(_) => {}
        }
    }

    assert!(queue.is_empty());
}

pub fn resolve_1(
    mut stack: ResMut<Stack>,
    mut battlefield: ResMut<Battlefield>,
    mut etb_events: EventWriter<EtbEvent>,
    mut commands: Commands,
    cards_in_stack: Query<
        (
            &StackId,
            &player::Controller,
            &player::Owner,
            &SpellEffects,
            &CastingModifiers,
            &CastingCost,
            &Colors,
            &CardTypes,
            &CardSubtypes,
            Option<&ModifyingTypes>,
            Option<&ModifyingSubtypes>,
            Option<&Targets>,
        ),
        (With<StackId>, Without<BattlefieldId>),
    >,
    mut cards_on_battlefield: Query<
        (
            Entity,
            &player::Controller,
            &CardTypes,
            &CardSubtypes,
            &CastingCost,
            &Colors,
            Option<&mut ModifyingPower>,
            Option<&mut ModifyingToughness>,
            Option<&mut ModifyingSubtypes>,
        ),
        (With<BattlefieldId>, Without<StackId>),
    >,
    type_modifiers: Query<&ModifyingTypeSet>,
    subtype_modifiers: Query<&ModifyingSubtypeSet>,
    static_abilities: Query<(&StaticAbilities, &player::Controller)>,
    active_abilities: Query<(&player::Controller, &ActiveAbility, Option<&Targets>)>,
) {
    let Some((_, entry)) = stack.entries.pop() else {
        return;
    };

    match entry {
        StackEntry::Spell(entity) => {
            let (_, spell_controller, spell_owner, effects, _, _, _, types, _, _, _, maybe_target) =
                cards_in_stack.get(entity).unwrap();
            commands
                .entity(entity)
                .remove::<StackId>()
                .insert(player::Controller::from(*spell_owner));
            commands
                .entity(entity)
                .insert(battlefield.next_graveyard_id());

            if Card::requires_target(effects) && maybe_target.is_none() {
                return;
            }

            for effect in effects.iter() {
                match effect {
                    SpellEffect::CounterSpell {
                        valid_target:
                            SpellTarget {
                                controller,
                                types,
                                subtypes,
                            },
                    } => {
                        resolve_counterspell(
                            maybe_target,
                            &cards_in_stack,
                            &mut stack,
                            &static_abilities,
                            controller,
                            spell_controller,
                            types,
                            &type_modifiers,
                            subtypes,
                            &subtype_modifiers,
                            &mut commands,
                            &mut battlefield,
                        );
                    }
                    SpellEffect::GainMana { mana: _ } => todo!(),
                    SpellEffect::BattlefieldModifier(modifier) => {
                        if !apply_battlefield_modifier(
                            &mut commands,
                            modifier,
                            &mut cards_on_battlefield,
                            spell_controller,
                            maybe_target,
                            None,
                        ) {
                            return;
                        }
                    }
                    SpellEffect::ControllerDrawCards(_) => todo!(),
                    SpellEffect::AddPowerToughnessToTarget(_) => todo!(),
                    SpellEffect::ModifyCreature(_) => todo!(),
                    SpellEffect::ExileTargetCreature => todo!(),
                    SpellEffect::ExileTargetCreatureManifestTopOfLibrary => todo!(),
                }
            }

            if Card::is_permanent(types) {
                etb_events.send(EtbEvent {
                    card: entity,
                    targets: None,
                });
            }
        }
        StackEntry::ActivatedAbility(ability) => {
            let (ability_controller, ability, maybe_target) =
                active_abilities.get(ability).unwrap();

            for effect in ability.effects.iter() {
                match effect {
                    ActivatedAbilityEffect::CounterSpell {
                        valid_target:
                            SpellTarget {
                                controller,
                                types,
                                subtypes,
                            },
                    } => {
                        resolve_counterspell(
                            maybe_target,
                            &cards_in_stack,
                            &mut stack,
                            &static_abilities,
                            controller,
                            ability_controller,
                            types,
                            &type_modifiers,
                            subtypes,
                            &subtype_modifiers,
                            &mut commands,
                            &mut battlefield,
                        );
                    }
                    ActivatedAbilityEffect::GainMana { mana: _ } => todo!(),
                    ActivatedAbilityEffect::BattlefieldModifier(modifier) => {
                        if !apply_battlefield_modifier(
                            &mut commands,
                            modifier,
                            &mut cards_on_battlefield,
                            ability_controller,
                            maybe_target,
                            Some(ability.source),
                        ) {
                            return;
                        }
                    }
                    ActivatedAbilityEffect::ControllerDrawCards(_) => todo!(),
                    ActivatedAbilityEffect::Equip(_) => todo!(),
                    ActivatedAbilityEffect::AddPowerToughnessToTarget(_) => todo!(),
                }
            }
        }
        StackEntry::TriggeredAbility(_) => todo!(),
    }
}

fn apply_battlefield_modifier(
    commands: &mut Commands,
    modifier: &crate::effects::BattlefieldModifier,
    cards_on_battlefield: &mut Query<
        (
            Entity,
            &player::Controller,
            &CardTypes,
            &CardSubtypes,
            &CastingCost,
            &Colors,
            Option<&mut ModifyingPower>,
            Option<&mut ModifyingToughness>,
            Option<&mut ModifyingSubtypes>,
        ),
        (With<BattlefieldId>, Without<StackId>),
    >,
    spell_controller: &player::Controller,
    maybe_target: Option<&Targets>,
    maybe_source: Option<Entity>,
) -> bool {
    let modifier_id = commands
        .spawn(modifier.clone())
        .insert(modifier.duration)
        .id();
    let controls_black_or_green =
        cards_on_battlefield
            .iter()
            .any(|(_, controller, _, _, cost, colors, _, _, _)| {
                if spell_controller != controller {
                    return false;
                }

                let colors = Card::colors(cost, colors);
                colors.contains(Color::Black) || colors.contains(Color::Green)
            });

    'cards: for (entity, _, types, subtypes, _, _, power_mod, toughness_mod, subtypes_mod) in
        cards_on_battlefield.iter_mut()
    {
        for restriction in modifier.restrictions().iter() {
            match restriction {
                Restriction::NotSelf => {
                    if maybe_source.is_some() && maybe_source.unwrap() == entity {
                        continue 'cards;
                    }
                }
                Restriction::SingleTarget => {
                    if maybe_target.is_none() || maybe_target.unwrap().len() != 1 {
                        return false;
                    }
                }
                Restriction::CreaturesOnly => {
                    if !types.contains(Type::Creature) {
                        continue 'cards;
                    }
                }
                Restriction::ControllerControlsBlackOrGreen => {
                    if !controls_black_or_green {
                        return false;
                    }
                }
            }
        }

        match &modifier.modifier {
            ModifyBattlefield::ModifyBasePowerToughness(modifier) => {
                if modifier.targets.is_disjoint(**subtypes) {
                    continue 'cards;
                }

                commands
                    .entity(modifier_id)
                    .insert(PowerModifier::SetBase(modifier.power));
                commands
                    .entity(modifier_id)
                    .insert(ToughnessModifier::SetBase(modifier.toughness));

                if let Some(mut power_mod) = power_mod {
                    power_mod.insert(modifier_id);
                } else {
                    commands
                        .entity(entity)
                        .insert(ModifyingPower::from(IndexSet::from([modifier_id])));
                }

                if let Some(mut toughness_mod) = toughness_mod {
                    toughness_mod.insert(modifier_id);
                } else {
                    commands
                        .entity(entity)
                        .insert(ModifyingToughness::from(IndexSet::from([modifier_id])));
                }
            }
            ModifyBattlefield::AddCreatureSubtypes(modifier) => {
                if modifier.targets.is_disjoint(**subtypes) {
                    continue 'cards;
                }

                commands
                    .entity(modifier_id)
                    .insert(ModifyingSubtypeSet::Adding(modifier.types));

                if let Some(mut subtypes_mod) = subtypes_mod {
                    subtypes_mod.insert(modifier_id);
                } else {
                    commands
                        .entity(entity)
                        .insert(ModifyingSubtypes::from(IndexSet::from([modifier_id])));
                }
            }
            ModifyBattlefield::RemoveAllSubtypes(_) => todo!(),
            ModifyBattlefield::AddPowerToughness(_) => todo!(),
            ModifyBattlefield::Vigilance(_) => todo!(),
        }
    }
    true
}

fn resolve_counterspell(
    maybe_target: Option<&Targets>,
    cards_in_stack: &Query<
        (
            &StackId,
            &player::Controller,
            &Owner,
            &SpellEffects,
            &CastingModifiers,
            &CastingCost,
            &Colors,
            &CardTypes,
            &CardSubtypes,
            Option<&ModifyingTypes>,
            Option<&ModifyingSubtypes>,
            Option<&Targets>,
        ),
        (With<StackId>, Without<BattlefieldId>),
    >,
    stack: &mut ResMut<Stack>,
    static_abilities: &Query<(&StaticAbilities, &player::Controller)>,
    controller: &Controller,
    spell_controller: &player::Controller,
    types: &enumset::EnumSet<Type>,
    type_modifiers: &Query<&ModifyingTypeSet>,
    subtypes: &enumset::EnumSet<crate::types::Subtype>,
    subtype_modifiers: &Query<&ModifyingSubtypeSet>,
    commands: &mut Commands,
    battlefield: &mut ResMut<Battlefield>,
) {
    let stack_targets = match maybe_target.expect("Validated target exists") {
        Targets::Stack(targets) => targets.clone(),
        Targets::Entities(targets) => targets
            .iter()
            .map(|target| *cards_in_stack.get(*target).unwrap().0)
            .collect::<Vec<_>>(),
        _ => {
            // Only the stack is a valid target for counterspells
            return;
        }
    };
    'targets: for stack_target in stack_targets {
        let target = stack
            .entries
            .get(&stack_target)
            .expect("Stack ids should always be valid");
        let entity_target = match target {
            StackEntry::Spell(target) => *target,
            _ => {
                // Only spells are a valid target for counterspells
                continue 'targets;
            }
        };

        let (
            _,
            target_controller,
            target_owner,
            _,
            casting_modifiers,
            casting_cost,
            colors,
            card_types,
            card_subtypes,
            modifying_types,
            modifying_subtypes,
            _,
        ) = cards_in_stack.get(entity_target).unwrap();

        if casting_modifiers.contains(CastingModifier::CannotBeCountered) {
            continue 'targets;
        }

        for (abilities, ability_controller) in static_abilities.iter() {
            for ability in abilities.iter() {
                match ability {
                    StaticAbility::GreenCannotBeCountered { controller } => {
                        if Card::colors(casting_cost, colors).contains(Color::Green) {
                            match controller {
                                Controller::Any => {
                                    continue 'targets;
                                }
                                Controller::You => {
                                    if ability_controller == target_controller {
                                        continue 'targets;
                                    }
                                }
                                Controller::Opponent => {
                                    if ability_controller != target_controller {
                                        continue 'targets;
                                    }
                                }
                            }
                        }
                    }
                    StaticAbility::Vigilance => {}
                    StaticAbility::BattlefieldModifier(_) => {}
                    StaticAbility::Enchant(_) => {}
                }
            }
        }

        match controller {
            Controller::Any => {}
            Controller::You => {
                if spell_controller != target_controller {
                    continue 'targets;
                }
            }
            Controller::Opponent => {
                if spell_controller == target_controller {
                    continue 'targets;
                }
            }
        }

        if !types.is_empty() {
            let card_types = modifying_types
                .map(|types| types.union(card_types, type_modifiers))
                .unwrap_or_else(|| Ok(**card_types))
                .unwrap();

            if card_types.intersection(*types).is_empty() {
                continue 'targets;
            }
        }

        if !subtypes.is_empty() {
            let card_types = modifying_subtypes
                .map(|types| types.union(card_subtypes, subtype_modifiers))
                .unwrap_or_else(|| Ok(**card_subtypes))
                .unwrap();

            if card_types.intersection(*subtypes).is_empty() {
                continue 'targets;
            }
        }

        commands
            .entity(entity_target)
            .remove::<StackId>()
            .insert(player::Controller::from(*target_owner));
        stack.entries.remove(&stack_target);

        commands
            .entity(entity_target)
            .insert(battlefield.next_graveyard_id());
    }
}
