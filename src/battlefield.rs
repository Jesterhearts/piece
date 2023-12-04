use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, EventWriter, Events},
    query::With,
    system::{Commands, Query, ResMut, Resource},
};

use crate::{
    abilities::{Copying, ETBAbility},
    card::{Card, ModifyingToughness, ModifyingTypeSet, ModifyingTypes, ToughnessModifier},
    types::Type,
    FollowupWork,
};

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

pub fn handle_sba(
    mut to_graveyard: EventWriter<PermanentToGraveyardEvent>,
    cards_on_battlefield: Query<
        (Entity, &Card, Option<&Copying>, Option<&ModifyingToughness>),
        With<BattlefieldId>,
    >,
    toughness_modifiers: Query<&ToughnessModifier>,
    cards: Query<&Card>,
) -> anyhow::Result<()> {
    for (e, card, copying, modifying_toughness) in cards_on_battlefield.iter() {
        let card = if let Some(copying) = copying {
            cards.get(**copying)?
        } else {
            card
        };

        let toughness = modifying_toughness
            .map(|modifier| modifier.toughness(card, &toughness_modifiers))
            .unwrap_or(Ok(card.toughness))?;

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
    cards: Query<&Card>,
    cards_on_battlefield: Query<(Entity, &Card, Option<&ModifyingTypes>), With<BattlefieldId>>,
    type_modifiers: Query<&ModifyingTypeSet>,
) -> anyhow::Result<()> {
    let etb_events = etb_events.drain().collect::<Vec<_>>();
    let graveyard_events = graveyard_events.drain().collect::<Vec<_>>();

    let mut events_to_add = vec![];
    let mut events_to_graveyard = vec![];

    let mut had_followup = false;

    for event in etb_events {
        let mut add_to_battlefield = true;
        for etb in cards.get(event.card)?.etb_abilities.iter() {
            match etb {
                ETBAbility::CopyOfAnyCreature => {
                    if event.targets.is_none() {
                        let mut targets = vec![];
                        for (entity, card, modifying_types) in cards_on_battlefield.iter() {
                            let types = modifying_types
                                .map(|types| types.union(card, &type_modifiers))
                                .unwrap_or_else(|| Ok(card.types))?;

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
            for etb in cards.get(event.card)?.etb_abilities.iter() {
                match etb {
                    ETBAbility::CopyOfAnyCreature => {
                        if let Some(targets) = event.targets.as_mut() {
                            assert!(targets.len() <= 1);
                            if let Some(target) = targets.pop() {
                                let card = cards.get(target)?;

                                commands.entity(event.card).insert(Copying(target));

                                for etb in card.etb_abilities.iter() {
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
