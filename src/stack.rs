use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, EventWriter, Events},
    query::With,
    system::{Commands, Query, ResMut, Resource},
};
use indexmap::IndexMap;

use crate::{
    abilities::StaticAbility,
    battlefield::{Battlefield, BattlefieldId, EtbEvent, GraveyardId},
    card::{
        Card, CardSubtypes, CardTypes, CastingModifier, CastingModifiers, Color, Colors,
        ModifyingSubtypeSet, ModifyingSubtypes, ModifyingTypeSet, ModifyingTypes, SpellEffects,
        StaticAbilities,
    },
    controller::Controller,
    cost::CastingCost,
    effects::{BattlefieldModifier, ModifyBattlefield, SpellEffect},
    player::{self, Owner},
    targets::SpellTarget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct StackId(usize);

#[derive(Debug, Clone, Component)]
pub enum Targets {
    Stack(Vec<StackId>),
    Battlefield(Vec<BattlefieldId>),
    Graveyard(Vec<GraveyardId>),
    Player(Vec<Owner>),
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
) -> anyhow::Result<()> {
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
                if cards.get(entity)?.contains(CastingModifier::SplitSecond) {
                    stack.split_second = true;
                    break;
                }
            }
            StackEntry::ActivatedAbility(_) => {}
            StackEntry::TriggeredAbility(_) => {}
        }
    }

    assert!(queue.is_empty());
    Ok(())
}

pub fn resolve_1(
    mut stack: ResMut<Stack>,
    mut results: EventWriter<StackResult>,
    mut commands: Commands,
    cards: Query<
        (
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
        With<StackId>,
    >,
    type_modifiers: Query<&ModifyingTypeSet>,
    subtype_modifiers: Query<&ModifyingSubtypeSet>,
    static_abilities: Query<(&StaticAbilities, &player::Controller)>,
    battlefield_modifiers: Query<(&BattlefieldModifier, &player::Controller)>,
) -> anyhow::Result<()> {
    let Some((_, entry)) = stack.entries.pop() else {
        return Ok(());
    };

    match entry {
        StackEntry::Spell(entity) => {
            let (spell_controller, spell_owner, effects, _, _, _, types, _, _, _, maybe_target) =
                cards.get(entity)?;
            commands
                .entity(entity)
                .remove::<StackId>()
                .insert(player::Controller::from(*spell_owner));
            results.send(StackResult::StackToGraveyard(entity));

            if Card::requires_target(effects) && maybe_target.is_none() {
                return Ok(());
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
                        let stack_targets = match maybe_target.expect("Validated target exists") {
                            Targets::Stack(targets) => targets,
                            _ => {
                                // Only the stack is a valid target for counterspells
                                return Ok(());
                            }
                        };

                        'targets: for stack_target in stack_targets {
                            let target = stack
                                .entries
                                .get(stack_target)
                                .expect("Stack ids should always be valid");
                            let entity_target = match target {
                                StackEntry::Spell(target) => *target,
                                _ => {
                                    // Only spells are a valid target for counterspells
                                    continue 'targets;
                                }
                            };

                            let (
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
                            ) = cards.get(entity_target)?;

                            if casting_modifiers.contains(CastingModifier::CannotBeCountered) {
                                continue 'targets;
                            }

                            for (abilities, ability_controller) in static_abilities.iter() {
                                for ability in abilities.iter() {
                                    match ability {
                                        StaticAbility::GreenCannotBeCountered { controller } => {
                                            if Card::colors(casting_cost, colors)
                                                .contains(Color::Green)
                                            {
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

                            for (modifier, _modifier_controller) in battlefield_modifiers.iter() {
                                match modifier.modifier {
                                    ModifyBattlefield::ModifyBasePowerToughness(_) => {}
                                    ModifyBattlefield::AddCreatureSubtypes(_) => {}
                                    ModifyBattlefield::RemoveAllSubtypes(_) => {}
                                    ModifyBattlefield::AddPowerToughness(_) => {}
                                    ModifyBattlefield::Vigilance(_) => {}
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
                                    .map(|types| types.union(card_types, &type_modifiers))
                                    .unwrap_or_else(|| Ok(**card_types))?;

                                if card_types.intersection(*types).is_empty() {
                                    continue 'targets;
                                }
                            }

                            if !subtypes.is_empty() {
                                let card_types = modifying_subtypes
                                    .map(|types| types.union(card_subtypes, &subtype_modifiers))
                                    .unwrap_or_else(|| Ok(**card_subtypes))?;

                                if card_types.intersection(*subtypes).is_empty() {
                                    continue 'targets;
                                }
                            }

                            commands
                                .entity(entity_target)
                                .remove::<StackId>()
                                .insert(player::Controller::from(*target_owner));
                            stack.entries.remove(stack_target);
                            results.send(StackResult::StackToGraveyard(entity_target));
                        }
                    }
                    SpellEffect::GainMana { mana: _ } => todo!(),
                    SpellEffect::BattlefieldModifier(_) => todo!(),
                    SpellEffect::ControllerDrawCards(_) => todo!(),
                    SpellEffect::AddPowerToughnessToTarget(_) => todo!(),
                    SpellEffect::ModifyCreature(_) => todo!(),
                    SpellEffect::ExileTargetCreature => todo!(),
                    SpellEffect::ExileTargetCreatureManifestTopOfLibrary => todo!(),
                }
            }

            if Card::is_permanent(types) {
                results.send(StackResult::StackToBattlefield(entity));
            }
        }
        StackEntry::ActivatedAbility(_) => todo!(),
        StackEntry::TriggeredAbility(_) => todo!(),
    }

    Ok(())
}

pub fn handle_results(
    mut queue: ResMut<Events<StackResult>>,
    mut etb_events: EventWriter<EtbEvent>,
    mut commands: Commands,
    mut battlefield: ResMut<Battlefield>,
) -> anyhow::Result<()> {
    for result in queue.drain() {
        match result {
            StackResult::StackToGraveyard(entity) => {
                commands
                    .entity(entity)
                    .insert(battlefield.next_graveyard_id());
            }
            StackResult::StackToBattlefield(entity) => {
                etb_events.send(EtbEvent {
                    card: entity,
                    targets: None,
                });
            }
        }
    }

    Ok(())
}
