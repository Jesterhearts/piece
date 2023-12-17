use std::collections::{HashMap, HashSet};

use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{With, Without},
};
use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    battlefield::{
        compute_deck_targets, ActionResult, Battlefield, PendingResults, UnresolvedAction,
        UnresolvedActionResult,
    },
    card::keyword::SplitSecond,
    controller::ControllerRestriction,
    effects::{BattlefieldModifier, Effect, EffectDuration, Mill, TutorLibrary},
    in_play::{
        AbilityId, CardId, Database, InStack, ModifierId, OnBattlefield, TriggerId, TriggerInStack,
    },
    player::{AllPlayers, Controller, Owner},
    targets::{Restriction, SpellTarget},
    types::Type,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Component)]
pub struct Settled;

#[derive(Debug, PartialEq, Eq, Clone, Component)]
pub struct Targets(pub Vec<Vec<ActiveTarget>>);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ActiveTarget {
    Stack { id: InStack },
    Battlefield { id: CardId },
    Graveyard { id: CardId },
    Library { id: CardId },
    Player { id: Owner },
}
impl ActiveTarget {
    pub fn display(&self, db: &mut Database, _all_players: &AllPlayers) -> String {
        match self {
            ActiveTarget::Stack { id } => {
                format!("Stack ({}): {}", id, id.title(db))
            }
            ActiveTarget::Battlefield { id } => {
                format!("{} - ({})", id.name(db), id.id(db),)
            }
            ActiveTarget::Graveyard { id } => {
                format!("{} - ({})", id.name(db), id.id(db),)
            }
            ActiveTarget::Library { id } => {
                format!("{} - ({})", id.name(db), id.id(db),)
            }
            ActiveTarget::Player { .. } => "Player".to_string(),
        }
    }

    fn id(&self) -> Option<CardId> {
        match self {
            ActiveTarget::Battlefield { id }
            | ActiveTarget::Graveyard { id }
            | ActiveTarget::Library { id } => Some(*id),
            ActiveTarget::Stack { .. } => None,
            ActiveTarget::Player { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Entry {
    Card(CardId),
    Ability {
        in_stack: AbilityId,
        source: AbilityId,
        card_source: CardId,
    },
    Trigger {
        in_stack: TriggerId,
        source: TriggerId,
        card_source: CardId,
    },
}

#[derive(Debug, Clone, Copy, Deref, DerefMut, Component)]
pub struct Mode(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackEntry {
    pub ty: Entry,
    pub targets: Vec<Vec<ActiveTarget>>,
    pub mode: Option<usize>,
}

impl StackEntry {
    pub fn remove_from_stack(&self, db: &mut Database) {
        match self.ty {
            Entry::Card(_) => {}
            Entry::Ability { in_stack, .. } => in_stack.remove_from_stack(db),
            Entry::Trigger { in_stack, .. } => in_stack.remove_from_stack(db),
        }
    }

    pub fn display(&self, db: &mut Database) -> String {
        match self.ty {
            Entry::Card(card) => card.name(db),
            Entry::Ability { source, .. } => {
                format!("{}: {}", source.source(db).name(db), source.text(db))
            }
            Entry::Trigger {
                source,
                card_source,
                ..
            } => {
                format!("{}: {}", card_source.name(db), source.short_text(db))
            }
        }
    }
}

#[derive(Debug)]
pub struct Stack;

impl Stack {
    pub fn split_second(db: &mut Database) -> bool {
        db.query_filtered::<(), (With<InStack>, With<SplitSecond>)>()
            .iter(db)
            .next()
            .is_some()
    }

    pub fn target_nth(db: &mut Database, nth: usize) -> ActiveTarget {
        let nth = db
            .cards
            .query::<&InStack>()
            .iter(&db.cards)
            .copied()
            .chain(
                db.abilities
                    .query::<&InStack>()
                    .iter(&db.abilities)
                    .copied(),
            )
            .sorted()
            .chain(
                db.triggers
                    .query::<&TriggerInStack>()
                    .iter(&db.triggers)
                    .map(|seq| (*seq).into()),
            )
            .collect_vec()[nth];

        ActiveTarget::Stack { id: nth }
    }

    pub fn in_stack(db: &mut Database) -> HashMap<InStack, Entry> {
        db.cards
            .query::<(&InStack, Entity)>()
            .iter(&db.cards)
            .map(|(seq, entity)| (*seq, Entry::Card(entity.into())))
            .chain(
                db.abilities
                    .query::<(&InStack, Entity, &AbilityId, &CardId)>()
                    .iter(&db.abilities)
                    .map(|(seq, entity, source, card_source)| {
                        (
                            *seq,
                            Entry::Ability {
                                in_stack: entity.into(),
                                source: *source,
                                card_source: *card_source,
                            },
                        )
                    }),
            )
            .chain(
                db.triggers
                    .query::<(&TriggerInStack, Entity)>()
                    .iter(&db.triggers)
                    .map(|(seq, entity)| {
                        (
                            (*seq).into(),
                            Entry::Trigger {
                                in_stack: TriggerId::from(entity),
                                source: seq.trigger,
                                card_source: seq.source,
                            },
                        )
                    }),
            )
            .sorted_by_key(|(seq, _)| *seq)
            .collect()
    }

    pub fn entries_unsettled(db: &mut Database) -> Vec<(InStack, StackEntry)> {
        db.cards
            .query_filtered::<(&InStack, Entity, &Targets, Option<&Mode>), Without<Settled>>()
            .iter(&db.cards)
            .map(|(seq, entity, targets, mode)| {
                (
                    *seq,
                    StackEntry {
                        ty: Entry::Card(entity.into()),
                        targets: targets.0.clone(),
                        mode: mode.map(|mode| mode.0),
                    },
                )
            })
            .chain(
                db.abilities
                    .query_filtered::<(
                        &InStack,
                        Entity,
                        &AbilityId,
                        &Targets,
                        &CardId,
                        Option<&Mode>,
                    ), Without<Settled>>()
                    .iter(&db.abilities)
                    .map(|(seq, entity, source, targets, card_source, mode)| {
                        (
                            *seq,
                            StackEntry {
                                ty: Entry::Ability {
                                    in_stack: entity.into(),
                                    source: *source,
                                    card_source: *card_source,
                                },
                                targets: targets.0.clone(),
                                mode: mode.map(|mode| mode.0),
                            },
                        )
                    }),
            )
            .chain(
                db.triggers
                    .query_filtered::<(&TriggerInStack, Entity, &Targets, Option<&Mode>), Without<Settled>>()
                    .iter(&db.triggers)
                    .map(|(seq, entity, targets, mode)| {
                        (
                            (*seq).into(),
                            StackEntry {
                                ty: Entry::Trigger {
                                    in_stack: TriggerId::from(entity),
                                    source: seq.trigger,
                                    card_source: seq.source,
                                },
                                targets: targets.0.clone(),
                                mode: mode.map(|mode| mode.0),
                            },
                        )
                    }),
            )
            .sorted_by_key(|(seq, _)| -*seq)
            .collect_vec()
    }

    pub fn entries(db: &mut Database) -> Vec<(InStack, StackEntry)> {
        db.cards
            .query::<(&InStack, Entity, &Targets, Option<&Mode>)>()
            .iter(&db.cards)
            .map(|(seq, entity, targets, mode)| {
                (
                    *seq,
                    StackEntry {
                        ty: Entry::Card(entity.into()),
                        targets: targets.0.clone(),
                        mode: mode.map(|mode| mode.0),
                    },
                )
            })
            .chain(
                db.abilities
                    .query::<(
                        &InStack,
                        Entity,
                        &AbilityId,
                        &Targets,
                        &CardId,
                        Option<&Mode>,
                    )>()
                    .iter(&db.abilities)
                    .map(|(seq, entity, source, targets, card_source, mode)| {
                        (
                            *seq,
                            StackEntry {
                                ty: Entry::Ability {
                                    in_stack: entity.into(),
                                    source: *source,
                                    card_source: *card_source,
                                },
                                targets: targets.0.clone(),
                                mode: mode.map(|mode| mode.0),
                            },
                        )
                    }),
            )
            .chain(
                db.triggers
                    .query::<(&TriggerInStack, Entity, &Targets, Option<&Mode>)>()
                    .iter(&db.triggers)
                    .map(|(seq, entity, targets, mode)| {
                        (
                            (*seq).into(),
                            StackEntry {
                                ty: Entry::Trigger {
                                    in_stack: TriggerId::from(entity),
                                    source: seq.trigger,
                                    card_source: seq.source,
                                },
                                targets: targets.0.clone(),
                                mode: mode.map(|mode| mode.0),
                            },
                        )
                    }),
            )
            .sorted_by_key(|(seq, _)| -*seq)
            .collect_vec()
    }

    fn pop(db: &mut Database) -> Option<StackEntry> {
        db.cards
            .query::<(&InStack, Entity, &Targets, Option<&Mode>)>()
            .iter(&db.cards)
            .map(|(seq, entity, targets, mode)| {
                (
                    *seq,
                    StackEntry {
                        ty: Entry::Card(entity.into()),
                        targets: targets.0.clone(),
                        mode: mode.map(|mode| mode.0),
                    },
                )
            })
            .chain(
                db.abilities
                    .query::<(
                        &InStack,
                        Entity,
                        &AbilityId,
                        &Targets,
                        &CardId,
                        Option<&Mode>,
                    )>()
                    .iter(&db.abilities)
                    .map(|(seq, entity, source, targets, card_source, mode)| {
                        (
                            *seq,
                            StackEntry {
                                ty: Entry::Ability {
                                    in_stack: entity.into(),
                                    source: *source,
                                    card_source: *card_source,
                                },
                                targets: targets.0.clone(),
                                mode: mode.map(|mode| mode.0),
                            },
                        )
                    }),
            )
            .chain(
                db.triggers
                    .query::<(Entity, &TriggerInStack, &Targets, Option<&Mode>)>()
                    .iter(&db.triggers)
                    .map(|(entity, seq, targets, mode)| {
                        (
                            (*seq).into(),
                            StackEntry {
                                ty: Entry::Trigger {
                                    in_stack: TriggerId::from(entity),
                                    source: seq.trigger,
                                    card_source: seq.source,
                                },
                                targets: targets.0.clone(),
                                mode: mode.map(|mode| mode.0),
                            },
                        )
                    }),
            )
            .sorted_by_key(|(seq, _)| *seq)
            .last()
            .map(|(_, entry)| {
                entry.remove_from_stack(db);
                entry
            })
    }

    pub fn is_empty(db: &mut Database) -> bool {
        db.cards
            .query_filtered::<(), With<InStack>>()
            .iter(&db.cards)
            .chain(
                db.abilities
                    .query_filtered::<(), With<InStack>>()
                    .iter(&db.abilities),
            )
            .chain(
                db.triggers
                    .query_filtered::<(), With<TriggerInStack>>()
                    .iter(&db.triggers),
            )
            .next()
            .is_none()
    }

    pub fn len(db: &mut Database) -> usize {
        db.cards
            .query_filtered::<(), With<InStack>>()
            .iter(&db.cards)
            .chain(
                db.abilities
                    .query_filtered::<(), With<InStack>>()
                    .iter(&db.abilities),
            )
            .chain(
                db.triggers
                    .query_filtered::<(), With<TriggerInStack>>()
                    .iter(&db.triggers),
            )
            .count()
    }

    pub fn settle(db: &mut Database) {
        let in_stack = Self::in_stack(db);
        for (_, entry) in in_stack.iter() {
            match entry {
                Entry::Card(card) => {
                    card.settle(db);
                }
                Entry::Ability { in_stack, .. } => {
                    in_stack.settle(db);
                }
                Entry::Trigger { in_stack, .. } => {
                    in_stack.settle(db);
                }
            }
        }
    }

    pub fn resolve_1(db: &mut Database) -> PendingResults {
        let Some(next) = Self::pop(db) else {
            return PendingResults::default();
        };

        Self::settle(db);
        let in_stack = Self::in_stack(db);

        let mut results = PendingResults::default();

        let (apply_to_self, effects, controller, resolving_card, source) = match next.ty {
            Entry::Card(card) => (
                false,
                card.effects(db),
                card.controller(db),
                Some(card),
                card,
            ),
            Entry::Ability { source, .. } => (
                source.apply_to_self(db),
                source.effects(db),
                source.controller(db),
                None,
                source.source(db),
            ),
            Entry::Trigger {
                source,
                card_source,
                ..
            } => (
                false,
                source.effects(db),
                card_source.controller(db),
                None,
                card_source,
            ),
        };

        let mut targets = next.targets.into_iter();
        for (effect, targets) in effects.into_iter().zip(&mut targets) {
            let effect = effect.into_effect(db, controller);
            if targets.len() != effect.needs_targets() {
                let creatures = Battlefield::creatures(db);
                let targets = source.targets_for_effect(db, controller, &effect, &creatures);
                results.push_unresolved(UnresolvedAction::new(
                    db,
                    Some(source),
                    UnresolvedActionResult::Effect(effect),
                    vec![targets],
                    false,
                ));
                continue;
            }

            match effect {
                Effect::CounterSpell {
                    target: restrictions,
                } => {
                    if !counter_spell(
                        db,
                        &in_stack,
                        controller,
                        targets,
                        &restrictions,
                        &mut results,
                    ) {
                        if let Some(resolving_card) = resolving_card {
                            return [ActionResult::StackToGraveyard(resolving_card)].into();
                        } else {
                            return PendingResults::default();
                        }
                    }
                }
                Effect::BattlefieldModifier(modifier) => {
                    if apply_to_self {
                        let modifier = ModifierId::upload_temporary_modifier(db, source, &modifier);
                        results.push_resolved(ActionResult::ApplyModifierToTarget {
                            modifier,
                            target: ActiveTarget::Battlefield { id: source },
                        });
                    } else {
                        results.push_resolved(ActionResult::ApplyToBattlefield(
                            ModifierId::upload_temporary_modifier(db, source, &modifier),
                        ));
                    }
                }
                Effect::ControllerDrawCards(count) => {
                    results.push_resolved(ActionResult::DrawCards {
                        target: controller,
                        count,
                    });
                }
                Effect::ModifyCreature(modifier) => {
                    let modifier = ModifierId::upload_temporary_modifier(db, source, &modifier);

                    let mut final_targets = vec![];
                    for target in targets {
                        match target {
                            ActiveTarget::Battlefield { .. } => {
                                final_targets.push(target);
                            }
                            _ => {
                                if let Some(resolving_card) = resolving_card {
                                    return [ActionResult::StackToGraveyard(resolving_card)].into();
                                } else {
                                    return PendingResults::default();
                                }
                            }
                        }
                    }

                    for target in final_targets {
                        results.push_resolved(ActionResult::ApplyModifierToTarget {
                            modifier,
                            target,
                        });
                    }
                }
                Effect::ExileTargetCreature => {
                    for target in targets {
                        match target {
                            ActiveTarget::Battlefield { id } => {
                                if !id.is_in_location::<OnBattlefield>(db) {
                                    // Permanent no longer on battlefield.
                                    if let Some(resolving_card) = resolving_card {
                                        return [ActionResult::StackToGraveyard(resolving_card)]
                                            .into();
                                    } else {
                                        return PendingResults::default();
                                    }
                                }

                                if !id.can_be_targeted(db, controller) {
                                    // Card is no longer a valid target.
                                    if let Some(resolving_card) = resolving_card {
                                        return [ActionResult::StackToGraveyard(resolving_card)]
                                            .into();
                                    } else {
                                        return PendingResults::default();
                                    }
                                }

                                if !id.types_intersect(db, &HashSet::from([Type::Creature])) {
                                    // Target isn't a creature

                                    if let Some(resolving_card) = resolving_card {
                                        return [ActionResult::StackToGraveyard(resolving_card)]
                                            .into();
                                    } else {
                                        return PendingResults::default();
                                    }
                                }

                                results.push_resolved(ActionResult::ExileTarget(target));
                            }
                            _ => {
                                if let Some(resolving_card) = resolving_card {
                                    return [ActionResult::StackToGraveyard(resolving_card)].into();
                                } else {
                                    return PendingResults::default();
                                }
                            }
                        }
                    }
                }
                Effect::ExileTargetCreatureManifestTopOfLibrary => {
                    for target in targets {
                        match target {
                            ActiveTarget::Battlefield { id } => {
                                if !id.is_in_location::<OnBattlefield>(db) {
                                    // Permanent no longer on battlefield.
                                    if let Some(resolving_card) = resolving_card {
                                        return [ActionResult::StackToGraveyard(resolving_card)]
                                            .into();
                                    } else {
                                        return PendingResults::default();
                                    }
                                }

                                if !id.can_be_targeted(db, controller) {
                                    // Card is no longer a valid target.
                                    if let Some(resolving_card) = resolving_card {
                                        return [ActionResult::StackToGraveyard(resolving_card)]
                                            .into();
                                    } else {
                                        return PendingResults::default();
                                    }
                                }

                                if !id.types_intersect(db, &HashSet::from([Type::Creature])) {
                                    // Target isn't a creature
                                    if let Some(resolving_card) = resolving_card {
                                        return [ActionResult::StackToGraveyard(resolving_card)]
                                            .into();
                                    } else {
                                        return PendingResults::default();
                                    }
                                }

                                results.push_resolved(ActionResult::ExileTarget(target));
                                results.push_resolved(ActionResult::ManifestTopOfLibrary(
                                    id.controller(db),
                                ));
                            }
                            _ => {
                                if let Some(resolving_card) = resolving_card {
                                    return [ActionResult::StackToGraveyard(resolving_card)].into();
                                } else {
                                    return PendingResults::default();
                                }
                            }
                        }
                    }
                }
                Effect::DealDamage(dmg) => {
                    for target in targets {
                        match target {
                            ActiveTarget::Battlefield { id } => {
                                if !id.is_in_location::<OnBattlefield>(db) {
                                    // Permanent no longer on battlefield.
                                    if let Some(resolving_card) = resolving_card {
                                        return [ActionResult::StackToGraveyard(resolving_card)]
                                            .into();
                                    } else {
                                        return PendingResults::default();
                                    }
                                }

                                if !id.can_be_targeted(db, controller) {
                                    // Card is no longer a valid target.
                                    if let Some(resolving_card) = resolving_card {
                                        return [ActionResult::StackToGraveyard(resolving_card)]
                                            .into();
                                    } else {
                                        return PendingResults::default();
                                    }
                                }

                                if !id.passes_restrictions(
                                    db,
                                    source,
                                    ControllerRestriction::Any,
                                    &dmg.restrictions,
                                ) {
                                    if let Some(resolving_card) = resolving_card {
                                        return [ActionResult::StackToGraveyard(resolving_card)]
                                            .into();
                                    } else {
                                        return PendingResults::default();
                                    }
                                }

                                results.push_resolved(ActionResult::DamageTarget {
                                    quantity: dmg.quantity,
                                    target,
                                });
                            }
                            ActiveTarget::Player { .. } => todo!(),
                            _ => {
                                if let Some(resolving_card) = resolving_card {
                                    return [ActionResult::StackToGraveyard(resolving_card)].into();
                                } else {
                                    return PendingResults::default();
                                }
                            }
                        }
                    }
                }
                Effect::Equip(modifiers) => {
                    match targets.into_iter().next().unwrap() {
                        ActiveTarget::Stack { .. } => {
                            // Can't equip things on the stack
                            if let Some(resolving_card) = resolving_card {
                                return [ActionResult::StackToGraveyard(resolving_card)].into();
                            } else {
                                return PendingResults::default();
                            }
                        }
                        ActiveTarget::Battlefield { id } => {
                            if !id.can_be_targeted(db, controller) {
                                // Card is not a valid target, spell fizzles.
                                if let Some(resolving_card) = resolving_card {
                                    return [ActionResult::StackToGraveyard(resolving_card)].into();
                                } else {
                                    return PendingResults::default();
                                }
                            }

                            // This is a hack. I hope equipement doesn't come with anthem effects.
                            source.deactivate_modifiers(db);
                            for modifier in modifiers {
                                let modifier = ModifierId::upload_temporary_modifier(
                                    db,
                                    source,
                                    &BattlefieldModifier {
                                        modifier: modifier.clone(),
                                        controller: ControllerRestriction::You,
                                        duration: EffectDuration::UntilSourceLeavesBattlefield,
                                        restrictions: vec![Restriction::OfType {
                                            types: HashSet::from([Type::Creature]),
                                            subtypes: Default::default(),
                                        }],
                                    },
                                );

                                results.push_resolved(ActionResult::ModifyCreatures {
                                    targets: vec![ActiveTarget::Battlefield { id }],
                                    modifier,
                                });
                            }
                        }
                        _ => {
                            if let Some(resolving_card) = resolving_card {
                                return [ActionResult::StackToGraveyard(resolving_card)].into();
                            } else {
                                return PendingResults::default();
                            }
                        }
                    }
                }
                Effect::CreateToken(token) => {
                    results.push_resolved(ActionResult::CreateToken {
                        source: controller,
                        token: token.clone(),
                    });
                }
                Effect::GainCounter(counter) => {
                    results.push_resolved(ActionResult::AddCounters {
                        target: source,
                        counter,
                        count: 1,
                    });
                }
                Effect::ControllerLosesLife(count) => {
                    results.push_resolved(ActionResult::LoseLife {
                        target: source.controller(db),
                        count,
                    })
                }
                Effect::Mill(Mill { count, .. }) => {
                    results.push_resolved(ActionResult::Mill { count, targets });
                }
                Effect::ReturnFromGraveyardToBattlefield(_) => {
                    results
                        .push_resolved(ActionResult::ReturnFromGraveyardToBattlefield { targets });
                }
                Effect::ReturnFromGraveyardToLibrary(_) => {
                    results.push_resolved(ActionResult::ReturnFromGraveyardToLibrary { targets });
                }
                Effect::TutorLibrary(TutorLibrary {
                    restrictions,
                    destination,
                    reveal,
                }) => {
                    let valid_targets = compute_deck_targets(db, controller, &restrictions)
                        .into_iter()
                        .map(|card| ActiveTarget::Library { id: card })
                        .collect_vec();

                    results.push_unresolved(UnresolvedAction::new(
                        db,
                        Some(source),
                        UnresolvedActionResult::Effect(Effect::TutorLibrary(TutorLibrary {
                            restrictions,
                            destination,
                            reveal,
                        })),
                        vec![valid_targets],
                        false,
                    ));
                }
                Effect::CopyOfAnyCreatureNonTargeting => unreachable!(),
                Effect::CreateTokenCopy { modifiers } => {
                    let target = targets.into_iter().next().unwrap();
                    let target = target.id();
                    results.push_resolved(ActionResult::CreateTokenCopyOf {
                        target: target.unwrap(),
                        modifiers,
                        controller: source.controller(db),
                    });
                }
                Effect::ReturnSelfToHand => {
                    source.move_to_hand(db);
                }
                Effect::RevealEachTopOfLibrary(reveal) => {
                    results.push_resolved(ActionResult::RevealEachTopOfLibrary(source, reveal));
                }
            }
        }

        if let Some(resolving_card) = resolving_card {
            if resolving_card.is_permanent(db) {
                results.push_resolved(ActionResult::AddToBattlefield(
                    resolving_card,
                    targets.next().and_then(|targets| {
                        targets.into_iter().find_map(|target| match target {
                            ActiveTarget::Battlefield { id } => Some(id),
                            _ => None,
                        })
                    }),
                ));
            } else {
                results.push_resolved(ActionResult::StackToGraveyard(resolving_card));
            }
        }

        results
    }

    pub fn move_etb_ability_to_stack(
        db: &mut Database,
        ability: AbilityId,
        source: CardId,
    ) -> PendingResults {
        let mut results = PendingResults::default();
        let creatures = Battlefield::creatures(db);

        results.push_resolved(ActionResult::AddAbilityToStack {
            ability,
            source,
            targets: source.targets_for_ability(db, ability, &creatures),
        });

        results
    }

    pub fn move_trigger_to_stack(
        db: &mut Database,
        trigger: TriggerId,
        source: CardId,
    ) -> PendingResults {
        let mut results = PendingResults::default();

        let mut targets = vec![];
        let creatures = Battlefield::creatures(db);
        let controller = source.controller(db);
        for effect in trigger.effects(db) {
            let effect = effect.into_effect(db, controller);
            targets.push(source.targets_for_effect(db, controller, &effect, &creatures));
        }

        results.push_resolved(ActionResult::AddTriggerToStack {
            trigger,
            source,
            targets,
        });

        results
    }

    pub fn move_card_to_stack(db: &mut Database, card: CardId) -> PendingResults {
        let mut results = PendingResults::default();

        if card.wants_targets(db).into_iter().sum::<usize>() > 0 {
            let valid_targets = card.valid_targets(db);
            results.push_unresolved(UnresolvedAction::new(
                db,
                Some(card),
                UnresolvedActionResult::AddCardToStack { choosing: 0 },
                valid_targets,
                false,
            ));
        } else {
            results.push_resolved(ActionResult::AddCardToStack {
                card,
                targets: vec![],
            })
        }
        results
    }
}

fn counter_spell(
    db: &mut Database,
    in_stack: &HashMap<InStack, Entry>,
    controller: Controller,
    targets: Vec<ActiveTarget>,
    restrictions: &SpellTarget,
    results: &mut PendingResults,
) -> bool {
    for target in targets {
        match target {
            ActiveTarget::Stack { id } => {
                let Some(maybe_target) = in_stack.get(&id) else {
                    // Spell has left the stack already
                    return false;
                };

                match maybe_target {
                    Entry::Card(maybe_target) => {
                        if !maybe_target.can_be_countered(db, controller, restrictions) {
                            // Spell is no longer a valid target.
                            return false;
                        }
                    }
                    Entry::Ability { .. } => {
                        // Vanilla counterspells can't counter activated abilities.
                        return false;
                    }
                    Entry::Trigger { .. } => {
                        // Vanilla counterspells can't counter triggered abilities.
                        return false;
                    }
                }

                // If we reach here, we know the spell can be countered.
                results.push_resolved(ActionResult::SpellCountered { id: *maybe_target });
            }
            ActiveTarget::Battlefield { .. } => {
                // Cards on the battlefield aren't valid targets of counterspells
                return false;
            }
            ActiveTarget::Player { .. } => {
                // Players aren't valid targets of counterspells
                return false;
            }
            ActiveTarget::Graveyard { .. } => return false,
            ActiveTarget::Library { .. } => return false,
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::{
        battlefield::{ActionResult, ResolutionResult},
        in_play::{CardId, Database},
        load_cards,
        player::AllPlayers,
        stack::Stack,
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let mut db = Database::default();
        let mut all_players = AllPlayers::default();
        let player = all_players.new_player("Player".to_string(), 20);
        let card1 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");

        card1.move_to_stack(&mut db, Default::default());

        let mut results = Stack::resolve_1(&mut db);

        assert_eq!(
            results,
            [ActionResult::AddToBattlefield(card1, None)].into()
        );
        let result = results.resolve(&mut db, &mut all_players, None);
        assert_eq!(result, ResolutionResult::Complete);

        assert!(Stack::is_empty(&mut db));

        Ok(())
    }
}
