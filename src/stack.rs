use std::collections::{HashMap, HashSet};

use bevy_ecs::{component::Component, entity::Entity, query::With};
use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    card::keyword::SplitSecond,
    controller::ControllerRestriction,
    effects::{BattlefieldModifier, Effect, EffectDuration},
    in_play::{
        AbilityId, CardId, Database, InStack, ModifierId, OnBattlefield, TriggerId, TriggerInStack,
    },
    player::{AllPlayers, Controller, Owner},
    targets::{Restriction, SpellTarget},
    types::Type,
};

#[derive(Debug, PartialEq, Eq, Clone, Component)]
pub struct Targets(pub Vec<ActiveTarget>);

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
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Entry {
    Card(CardId),
    Ability {
        in_stack: AbilityId,
        source: AbilityId,
    },
    Trigger {
        in_stack: TriggerId,
        source: TriggerId,
        card_source: CardId,
    },
}

#[derive(Debug, Clone, Copy, Deref, DerefMut, Component)]
pub struct Mode(pub usize);

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub ty: Entry,
    pub targets: Vec<ActiveTarget>,
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
                    .query::<(&InStack, Entity, &AbilityId)>()
                    .iter(&db.abilities)
                    .map(|(seq, entity, source)| {
                        (
                            *seq,
                            Entry::Ability {
                                in_stack: entity.into(),
                                source: *source,
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
                    .query::<(&InStack, Entity, &AbilityId, &Targets, Option<&Mode>)>()
                    .iter(&db.abilities)
                    .map(|(seq, entity, source, targets, mode)| {
                        (
                            *seq,
                            StackEntry {
                                ty: Entry::Ability {
                                    in_stack: entity.into(),
                                    source: *source,
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
                    .query::<(&InStack, Entity, &AbilityId, &Targets, Option<&Mode>)>()
                    .iter(&db.abilities)
                    .map(|(seq, entity, source, targets, mode)| {
                        (
                            *seq,
                            StackEntry {
                                ty: Entry::Ability {
                                    in_stack: entity.into(),
                                    source: *source,
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

    #[must_use]
    pub fn resolve_1(db: &mut Database) -> Vec<ActionResult> {
        let Some(next) = Self::pop(db) else {
            return vec![];
        };

        let in_stack = Self::in_stack(db);

        let mut results = vec![];

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

        for effect in effects {
            match effect.into_effect(db, controller) {
                Effect::CounterSpell {
                    target: restrictions,
                } => {
                    if !counter_spell(
                        db,
                        &in_stack,
                        controller,
                        &next.targets,
                        &restrictions,
                        &mut results,
                    ) {
                        if let Some(resolving_card) = resolving_card {
                            return vec![ActionResult::StackToGraveyard(resolving_card)];
                        } else {
                            return vec![];
                        }
                    }
                }
                Effect::BattlefieldModifier(modifier) => {
                    if apply_to_self {
                        let modifier = ModifierId::upload_temporary_modifier(db, source, &modifier);
                        results.push(ActionResult::ApplyModifierToTarget {
                            modifier,
                            target: ActiveTarget::Battlefield { id: source },
                        });
                    } else {
                        results.push(ActionResult::ApplyToBattlefield(
                            ModifierId::upload_temporary_modifier(db, source, &modifier),
                        ));
                    }
                }
                Effect::ControllerDrawCards(count) => {
                    results.push(ActionResult::DrawCards {
                        target: controller,
                        count,
                    });
                }
                Effect::ModifyCreature(modifier) => {
                    let modifier = ModifierId::upload_temporary_modifier(db, source, &modifier);

                    let mut targets = vec![];
                    for target in next.targets.iter() {
                        match target {
                            ActiveTarget::Battlefield { .. } => {
                                targets.push(target);
                            }
                            _ => {
                                if let Some(resolving_card) = resolving_card {
                                    return vec![ActionResult::StackToGraveyard(resolving_card)];
                                } else {
                                    return vec![];
                                }
                            }
                        }
                    }

                    for target in targets {
                        results.push(ActionResult::ApplyModifierToTarget {
                            modifier,
                            target: *target,
                        });
                    }
                }
                Effect::ExileTargetCreature => {
                    for target in next.targets.iter() {
                        match target {
                            ActiveTarget::Battlefield { id } => {
                                if !id.is_in_location::<OnBattlefield>(db) {
                                    // Permanent no longer on battlefield.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![ActionResult::StackToGraveyard(
                                            resolving_card,
                                        )];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.can_be_targeted(db, controller) {
                                    // Card is no longer a valid target.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![ActionResult::StackToGraveyard(
                                            resolving_card,
                                        )];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.types_intersect(db, &HashSet::from([Type::Creature])) {
                                    // Target isn't a creature

                                    if let Some(resolving_card) = resolving_card {
                                        return vec![ActionResult::StackToGraveyard(
                                            resolving_card,
                                        )];
                                    } else {
                                        return vec![];
                                    }
                                }

                                results.push(ActionResult::ExileTarget(*target));
                            }
                            _ => {
                                if let Some(resolving_card) = resolving_card {
                                    return vec![ActionResult::StackToGraveyard(resolving_card)];
                                } else {
                                    return vec![];
                                }
                            }
                        }
                    }
                }
                Effect::ExileTargetCreatureManifestTopOfLibrary => {
                    for target in next.targets.iter() {
                        match target {
                            ActiveTarget::Battlefield { id } => {
                                if !id.is_in_location::<OnBattlefield>(db) {
                                    // Permanent no longer on battlefield.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![ActionResult::StackToGraveyard(
                                            resolving_card,
                                        )];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.can_be_targeted(db, controller) {
                                    // Card is no longer a valid target.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![ActionResult::StackToGraveyard(
                                            resolving_card,
                                        )];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.types_intersect(db, &HashSet::from([Type::Creature])) {
                                    // Target isn't a creature
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![ActionResult::StackToGraveyard(
                                            resolving_card,
                                        )];
                                    } else {
                                        return vec![];
                                    }
                                }

                                results.push(ActionResult::ExileTarget(*target));
                                results.push(ActionResult::ManifestTopOfLibrary(id.controller(db)));
                            }
                            _ => {
                                if let Some(resolving_card) = resolving_card {
                                    return vec![ActionResult::StackToGraveyard(resolving_card)];
                                } else {
                                    return vec![];
                                }
                            }
                        }
                    }
                }
                Effect::DealDamage(dmg) => {
                    for target in next.targets.iter() {
                        match target {
                            ActiveTarget::Battlefield { id } => {
                                if !id.is_in_location::<OnBattlefield>(db) {
                                    // Permanent no longer on battlefield.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![ActionResult::StackToGraveyard(
                                            resolving_card,
                                        )];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.can_be_targeted(db, controller) {
                                    // Card is no longer a valid target.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![ActionResult::StackToGraveyard(
                                            resolving_card,
                                        )];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.passes_restrictions(
                                    db,
                                    source,
                                    controller,
                                    ControllerRestriction::Any,
                                    &dmg.restrictions,
                                ) {
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![ActionResult::StackToGraveyard(
                                            resolving_card,
                                        )];
                                    } else {
                                        return vec![];
                                    }
                                }

                                results.push(ActionResult::DamageTarget {
                                    quantity: dmg.quantity,
                                    target: *target,
                                });
                            }
                            ActiveTarget::Player { .. } => todo!(),
                            _ => {
                                if let Some(resolving_card) = resolving_card {
                                    return vec![ActionResult::StackToGraveyard(resolving_card)];
                                } else {
                                    return vec![];
                                }
                            }
                        }
                    }
                }
                Effect::Equip(modifiers) => {
                    if next.targets.is_empty() {
                        // Effect fizzles due to lack of target.
                        return vec![];
                    }

                    assert_eq!(next.targets.len(), 1);

                    match next.targets.first().unwrap() {
                        ActiveTarget::Stack { .. } => {
                            // Can't equip things on the stack
                            if let Some(resolving_card) = resolving_card {
                                return vec![ActionResult::StackToGraveyard(resolving_card)];
                            } else {
                                return vec![];
                            }
                        }
                        ActiveTarget::Battlefield { id } => {
                            if !id.can_be_targeted(db, controller) {
                                // Card is not a valid target, spell fizzles.
                                if let Some(resolving_card) = resolving_card {
                                    return vec![ActionResult::StackToGraveyard(resolving_card)];
                                } else {
                                    return vec![];
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

                                results.push(ActionResult::ModifyCreatures {
                                    targets: vec![ActiveTarget::Battlefield { id: *id }],
                                    modifier,
                                });
                            }
                        }
                        _ => {
                            if let Some(resolving_card) = resolving_card {
                                return vec![ActionResult::StackToGraveyard(resolving_card)];
                            } else {
                                return vec![];
                            }
                        }
                    }
                }
                Effect::CreateToken(token) => {
                    results.push(ActionResult::CreateToken {
                        source: controller,
                        token: token.clone(),
                    });
                }
                Effect::GainCounter(counter) => {
                    results.push(ActionResult::AddCounters {
                        target: source,
                        counter,
                        count: 1,
                    });
                }
                Effect::ControllerLosesLife(count) => results.push(ActionResult::LoseLife {
                    target: source.controller(db),
                    count,
                }),
                Effect::Mill(_) => todo!(),
                Effect::ReturnFromGraveyardToBattlefield(_) => todo!(),
                Effect::ReturnFromGraveyardToLibrary(_) => todo!(),
                Effect::TutorLibrary(_) => todo!(),
                Effect::CopyOfAnyCreatureNonTargeting => todo!(),
            }
        }

        if let Some(resolving_card) = resolving_card {
            if resolving_card.is_permanent(db) {
                results.push(ActionResult::AddToBattlefield(resolving_card));
            } else {
                results.push(ActionResult::StackToGraveyard(resolving_card));
            }
        }

        results
    }
}

fn counter_spell(
    db: &mut Database,
    in_stack: &HashMap<InStack, Entry>,
    controller: Controller,
    targets: &Vec<ActiveTarget>,
    restrictions: &SpellTarget,
    result: &mut Vec<ActionResult>,
) -> bool {
    if targets.is_empty() {
        return false;
    }
    for target in targets.iter() {
        match target {
            ActiveTarget::Stack { id } => {
                let Some(maybe_target) = in_stack.get(id) else {
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
                result.push(ActionResult::SpellCountered { id: *maybe_target });
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
        battlefield::{ActionResult, Battlefield, PendingResults},
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
        let player = all_players.new_player(20);
        let card1 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");

        card1.move_to_stack(&mut db, Default::default());

        let results = Stack::resolve_1(&mut db);

        assert_eq!(results, [ActionResult::AddToBattlefield(card1)]);
        let results = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
        assert_eq!(results, PendingResults::default());

        assert!(Stack::is_empty(&mut db));

        Ok(())
    }
}
