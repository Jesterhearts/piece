use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, EventWriter, Events},
    query::With,
    system::{Commands, Query, ResMut, Resource},
};

use crate::{
    abilities::{Copying, ETBAbility},
    activated_ability::ActiveAbility,
    card::{
        ActivatedAbilities, CardTypes, ETBAbilities, ModifyingToughness, ModifyingTypeSet,
        ModifyingTypes, Toughness, ToughnessModifier,
    },
    cost::AdditionalCost,
    player::{Controller, ManaPool},
    stack::{AddToStackEvent, StackEntry, StackId, Targets},
    types::Type,
    FollowupWork,
};

#[derive(Debug, Clone, Event)]
pub struct ActivateAbilityEvent {
    pub card: Entity,
    pub index: usize,
    pub targets: Vec<Entity>,
}

#[derive(Debug, Clone, Event)]
pub struct EtbEvent {
    pub card: Entity,
    pub targets: Option<Vec<Entity>>,
}

#[derive(Debug, Event)]
pub struct PermanentToGraveyardEvent {
    pub card: Entity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct Tapped;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct BattlefieldId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct GraveyardId(usize);

#[derive(Debug, Default, Resource)]
pub struct Battlefield {
    next_id: usize,
}

impl Battlefield {
    pub fn next_id(&mut self) -> BattlefieldId {
        let id = self.next_id;
        self.next_id += 1;
        BattlefieldId(id)
    }

    pub fn next_graveyard_id(&mut self) -> GraveyardId {
        let id = self.next_id;
        self.next_id += 1;
        GraveyardId(id)
    }
}

pub fn activate_ability(
    mut add_to_stack: EventWriter<AddToStackEvent>,
    mut events: ResMut<Events<ActivateAbilityEvent>>,
    mut graveyard_events: EventWriter<PermanentToGraveyardEvent>,
    mut commands: Commands,
    cards: Query<(&ActivatedAbilities, &Controller, Option<&Tapped>), With<BattlefieldId>>,
    stack: Query<&StackId>,
    mut mana_pools: Query<&mut ManaPool>,
) -> anyhow::Result<()> {
    assert!(events.len() <= 1);
    if let Some(ActivateAbilityEvent {
        card: card_entity,
        index,
        targets,
    }) = events.drain().last()
    {
        let (activated_abilities, controller, tapped) = cards.get(card_entity)?;
        let ability = &activated_abilities[index];
        let mut mana = mana_pools.get_mut(controller.0)?;

        let mut costs: Vec<Box<dyn FnOnce(&mut Commands)>> = vec![];

        if ability.cost.tap {
            if tapped.is_some() {
                return Ok(());
            }
            costs.push(Box::new(|commands| {
                commands.entity(card_entity).insert(Tapped);
            }));
        } else if ability.cost.untap {
            if tapped.is_none() {
                return Ok(());
            }
            costs.push(Box::new(|commands| {
                commands.entity(card_entity).remove::<Tapped>();
            }));
        }

        let old_mana = *mana;

        for cost in ability.cost.mana_cost.iter() {
            if !mana.spend(*cost) {
                *mana = old_mana;
                return Ok(());
            }
        }

        if ability
            .cost
            .additional_cost
            .contains(AdditionalCost::SacrificeThis)
        {
            costs.push(Box::new(|_commands| {
                graveyard_events.send(PermanentToGraveyardEvent { card: card_entity });
            }));
        }

        let abilty = commands
            .spawn(ActiveAbility {
                effects: ability.effects.clone(),
            })
            .id();

        add_to_stack.send(AddToStackEvent {
            entry: StackEntry::ActivatedAbility(abilty),
            target: Some(Targets::Stack(
                targets
                    .into_iter()
                    .map(|target| Ok(stack.get(target).map(|id| *id)?))
                    .collect::<anyhow::Result<_>>()?,
            )),
        });

        for cost in costs {
            cost(&mut commands)
        }
    }

    Ok(())
}

pub fn handle_sba(
    mut to_graveyard: EventWriter<PermanentToGraveyardEvent>,
    cards_on_battlefield: Query<
        (
            Entity,
            &Toughness,
            Option<&Copying>,
            Option<&ModifyingToughness>,
        ),
        With<BattlefieldId>,
    >,
    toughness_modifiers: Query<&ToughnessModifier>,
    cards: Query<&Toughness>,
) -> anyhow::Result<()> {
    for (e, toughness, copying, modifying_toughness) in cards_on_battlefield.iter() {
        let toughness = if let Some(copying) = copying {
            cards.get(**copying)?
        } else {
            toughness
        };

        let toughness = modifying_toughness
            .map(|modifier| modifier.toughness(toughness, &toughness_modifiers))
            .unwrap_or(Ok(**toughness))?;

        if let Some(toughness) = toughness {
            if toughness <= 0 {
                to_graveyard.send(PermanentToGraveyardEvent { card: e })
            }
        }
    }

    Ok(())
}

pub fn handle_events(
    mut etb_events: ResMut<Events<EtbEvent>>,
    mut graveyard_events: ResMut<Events<PermanentToGraveyardEvent>>,
    mut followup_work: EventWriter<FollowupWork>,
    mut battlefield: ResMut<Battlefield>,
    mut commands: Commands,
    etb_abilities: Query<&ETBAbilities>,
    cards_on_battlefield: Query<(Entity, &CardTypes, Option<&ModifyingTypes>), With<BattlefieldId>>,
    type_modifiers: Query<&ModifyingTypeSet>,
) -> anyhow::Result<()> {
    let etb_events = etb_events.drain().collect::<Vec<_>>();
    let graveyard_events = graveyard_events.drain().collect::<Vec<_>>();

    let mut events_to_add = vec![];
    let mut events_to_graveyard = vec![];

    let mut had_followup = false;

    for event in etb_events {
        let mut add_to_battlefield = true;
        for etb in etb_abilities.get(event.card)?.iter() {
            match etb {
                ETBAbility::CopyOfAnyCreature => {
                    if event.targets.is_none() {
                        let mut targets = vec![];
                        for (entity, card_types, modifying_types) in cards_on_battlefield.iter() {
                            let types = modifying_types
                                .map(|types| types.union(card_types, &type_modifiers))
                                .unwrap_or_else(|| Ok(**card_types))?;

                            if types.contains(Type::Creature) {
                                targets.push(entity);
                            }
                        }

                        add_to_battlefield = false;
                        had_followup = true;
                        followup_work.send(FollowupWork::ChooseTargetThenEtb {
                            valid_targets: targets,
                            targets_for: event.card,
                            up_to: 1,
                        });
                    }
                }
            }
        }

        if add_to_battlefield {
            events_to_add.push(event);
        }
    }

    for event in graveyard_events {
        events_to_graveyard.push(event);
    }

    if had_followup {
        followup_work.send(FollowupWork::Etb {
            events: events_to_add,
        });
        followup_work.send(FollowupWork::Graveyard {
            events: events_to_graveyard,
        });
    } else {
        for mut event in events_to_add {
            for etb in etb_abilities.get(event.card)?.iter() {
                match etb {
                    ETBAbility::CopyOfAnyCreature => {
                        if let Some(targets) = event.targets.as_mut() {
                            assert!(targets.len() <= 1);
                            if let Some(target) = targets.pop() {
                                let etb_abilities = etb_abilities.get(target)?;

                                commands.entity(event.card).insert(Copying(target));

                                for etb in etb_abilities.iter() {
                                    match etb {
                                        ETBAbility::CopyOfAnyCreature => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }

            commands.entity(event.card).insert(battlefield.next_id());
        }

        for event in events_to_graveyard {
            commands
                .entity(event.card)
                .remove::<BattlefieldId>()
                .insert(battlefield.next_graveyard_id());
        }
    }

    Ok(())
}
