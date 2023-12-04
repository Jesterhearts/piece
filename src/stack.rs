use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, EventWriter, Events},
    query::With,
    system::{Commands, Query, ResMut, Resource},
};
use indexmap::IndexMap;

use crate::{
    battlefield::{Battlefield, BattlefieldId, GraveyardId},
    card::{
        Card, CastingModifier, ModifyingSubtypeSet, ModifyingSubtypes, ModifyingTypeSet,
        ModifyingTypes,
    },
    controller::Controller,
    effects::SpellEffect,
    player::{self, PlayerId},
    targets::SpellTarget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct StackId(usize);

#[derive(Debug, Clone, Copy, Component)]
pub enum Target {
    Stack(StackId),
    Battlefield(BattlefieldId),
    Graveyard(GraveyardId),
    Player(PlayerId),
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
    CardToGraveyard(Entity),
}

#[derive(Debug, Event)]
pub struct AddToStackEvent {
    pub entry: StackEntry,
    pub target: Option<Target>,
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

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn add_to_stack(
    mut stack: ResMut<Stack>,
    mut commands: Commands,
    mut queue: ResMut<Events<AddToStackEvent>>,
    cards: Query<&Card>,
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
                if cards
                    .get(entity)?
                    .casting_modifiers
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
    Ok(())
}

pub fn resolve_1(
    mut stack: ResMut<Stack>,
    mut results: EventWriter<StackResult>,
    mut commands: Commands,
    cards: Query<
        (
            &Card,
            &player::Controller,
            &player::Owner,
            Option<&ModifyingTypes>,
            Option<&ModifyingSubtypes>,
            Option<&Target>,
        ),
        With<StackId>,
    >,
    type_modifiers: Query<&ModifyingTypeSet>,
    subtype_modifiers: Query<&ModifyingSubtypeSet>,
) -> anyhow::Result<()> {
    let Some((_, entry)) = stack.entries.pop() else {
        return Ok(());
    };

    match entry {
        StackEntry::Spell(entity) => {
            let (card, spell_controller, spell_owner, _, _, maybe_target) = cards.get(entity)?;
            commands
                .entity(entity)
                .remove::<StackId>()
                .remove::<player::Controller>()
                .insert(player::Controller::from(*spell_owner));
            results.send(StackResult::CardToGraveyard(entity));

            if card.requires_target() && maybe_target.is_none() {
                return Ok(());
            }

            for effect in card.effects.iter() {
                match effect {
                    SpellEffect::CounterSpell {
                        valid_target:
                            SpellTarget {
                                controller,
                                types,
                                subtypes,
                            },
                    } => {
                        let stack_target = match maybe_target.expect("Validated target exists") {
                            Target::Stack(target) => target,
                            _ => {
                                // Only the stack is a valid target for counterspells
                                return Ok(());
                            }
                        };

                        let target = stack
                            .entries
                            .get(stack_target)
                            .expect("Stack ids should always be valid");
                        let entity_target = match target {
                            StackEntry::Spell(target) => *target,
                            _ => {
                                // Only spells are a valid target for counterspells
                                return Ok(());
                            }
                        };

                        let (
                            target_card,
                            target_controller,
                            target_owner,
                            modifying_types,
                            modifying_subtypes,
                            _,
                        ) = cards.get(entity_target)?;

                        match controller {
                            Controller::Any => {}
                            Controller::You => {
                                if spell_controller != target_controller {
                                    return Ok(());
                                }
                            }
                            Controller::Opponent => {
                                if spell_controller == target_controller {
                                    return Ok(());
                                }
                            }
                        }

                        if !types.is_empty() {
                            let card_types = modifying_types
                                .map(|types| types.union(target_card, &type_modifiers))
                                .unwrap_or_else(|| Ok(target_card.types))?;

                            if card_types.intersection(*types).is_empty() {
                                return Ok(());
                            }
                        }

                        if !subtypes.is_empty() {
                            let card_types = modifying_subtypes
                                .map(|types| types.union(target_card, &subtype_modifiers))
                                .unwrap_or_else(|| Ok(target_card.subtypes))?;

                            if card_types.intersection(*subtypes).is_empty() {
                                return Ok(());
                            }
                        }

                        commands
                            .entity(entity_target)
                            .remove::<StackId>()
                            .remove::<player::Controller>()
                            .insert(player::Controller::from(*target_owner));
                        stack.entries.remove(stack_target);
                        results.send(StackResult::CardToGraveyard(entity_target));
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
        }
        StackEntry::ActivatedAbility(_) => todo!(),
        StackEntry::TriggeredAbility(_) => todo!(),
    }

    Ok(())
}

pub fn handle_results(
    mut queue: ResMut<Events<StackResult>>,
    mut commands: Commands,
    mut battlefield: ResMut<Battlefield>,
) -> anyhow::Result<()> {
    for result in queue.drain() {
        match result {
            StackResult::CardToGraveyard(entity) => {
                commands
                    .entity(entity)
                    .insert(battlefield.next_graveyard_id());
            }
        }
    }

    Ok(())
}
