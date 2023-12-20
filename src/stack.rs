use std::collections::{HashMap, HashSet};

use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{With, Without},
};
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::{
        compute_deck_targets, ActionResult, ChooseTargets, EffectOrAura, PayCost, PendingResults,
        Source, SpendMana,
    },
    card::keyword::SplitSecond,
    controller::ControllerRestriction,
    effects::{
        BattlefieldModifier, Effect, EffectDuration, ForEachManaOfSource, Mill, TutorLibrary,
    },
    in_play::{
        cast_from, AbilityId, CardId, CastFrom, Database, InStack, ModifierId, TriggerId,
        TriggerInStack,
    },
    player::{AllPlayers, Owner},
    targets::Restriction,
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

        let mut results = PendingResults::new(Source::Card(source));
        results.apply_in_stages();

        let mut targets = next.targets.into_iter();
        for (effect, targets) in effects
            .into_iter()
            .zip((&mut targets).chain(std::iter::repeat(vec![])))
        {
            let effect = effect.into_effect(db, controller);
            if targets.len() != effect.needs_targets() && effect.needs_targets() != 0 {
                let valid_targets = source.targets_for_effect(db, controller, &effect);
                results.push_choose_targets(ChooseTargets::new(
                    EffectOrAura::Effect(effect),
                    valid_targets,
                ));
                continue;
            }

            if effect.wants_targets() > 0 {
                let valid_targets = source
                    .targets_for_effect(db, controller, &effect)
                    .into_iter()
                    .collect::<HashSet<_>>();
                if !targets.iter().all(|target| valid_targets.contains(target)) {
                    if let Some(resolving_card) = resolving_card {
                        return [ActionResult::StackToGraveyard(resolving_card)].into();
                    } else {
                        return PendingResults::default();
                    }
                }
            }

            match effect {
                Effect::CounterSpell { .. } => {
                    let in_stack = Self::in_stack(db);
                    let in_stack = &in_stack;
                    let results: &mut PendingResults = &mut results;
                    for target in targets {
                        let ActiveTarget::Stack { id } = target else {
                            unreachable!()
                        };

                        results.push_settled(ActionResult::SpellCountered {
                            id: *in_stack.get(&id).unwrap(),
                        });
                    }
                }
                Effect::BattlefieldModifier(modifier) => {
                    if apply_to_self {
                        let modifier = ModifierId::upload_temporary_modifier(db, source, &modifier);
                        results.push_settled(ActionResult::ModifyCreatures {
                            modifier,
                            targets: vec![ActiveTarget::Battlefield { id: source }],
                        });
                    } else {
                        results.push_settled(ActionResult::ApplyToBattlefield(
                            ModifierId::upload_temporary_modifier(db, source, &modifier),
                        ));
                    }
                }
                Effect::ControllerDrawCards(count) => {
                    results.push_settled(ActionResult::DrawCards {
                        target: controller,
                        count,
                    });
                }
                Effect::ModifyTarget(modifier) => {
                    let mut final_targets = vec![];
                    for target in targets {
                        match target {
                            ActiveTarget::Battlefield { .. } => {
                                final_targets.push(target);
                            }
                            ActiveTarget::Graveyard { .. } => {
                                final_targets.push(target);
                            }
                            _ => unreachable!(),
                        }
                    }

                    let modifier = match modifier.duration {
                        EffectDuration::UntilTargetLeavesBattlefield => {
                            ModifierId::upload_temporary_modifier(
                                db,
                                final_targets.iter().exactly_one().unwrap().id().unwrap(),
                                &modifier,
                            )
                        }
                        _ => ModifierId::upload_temporary_modifier(db, source, &modifier),
                    };

                    results.push_settled(ActionResult::ModifyCreatures {
                        targets: final_targets,
                        modifier,
                    });
                }
                Effect::ExileTargetCreature => {
                    for target in targets {
                        results.push_settled(ActionResult::ExileTarget(target));
                    }
                }
                Effect::ExileTargetCreatureManifestTopOfLibrary => {
                    for target in targets {
                        results.push_settled(ActionResult::ExileTarget(target));
                        results.push_settled(ActionResult::ManifestTopOfLibrary(
                            target.id().unwrap().controller(db),
                        ));
                    }
                }
                Effect::DealDamage(dmg) => {
                    for target in targets {
                        results.push_settled(ActionResult::DamageTarget {
                            quantity: dmg.quantity,
                            target,
                        });
                    }
                }
                Effect::TargetToTopOfLibrary { .. } => {
                    for target in targets {
                        results
                            .push_settled(ActionResult::ReturnFromBattlefieldToLibrary { target });
                    }
                }
                Effect::Equip(modifiers) => {
                    let target = targets.into_iter().exactly_one().unwrap();
                    // This is a hack. I hope equipement doesn't come with anthem effects.
                    // It probably works even so.
                    source.deactivate_modifiers(db);
                    source.activate_modifiers(db);
                    for modifier in modifiers {
                        let modifier = ModifierId::upload_temporary_modifier(
                            db,
                            source,
                            &BattlefieldModifier {
                                modifier: modifier.clone(),
                                controller: ControllerRestriction::You,
                                duration: EffectDuration::UntilSourceLeavesBattlefield,
                                restrictions: vec![Restriction::OfType {
                                    types: IndexSet::from([Type::Creature]),
                                    subtypes: Default::default(),
                                }],
                            },
                        );

                        results.push_settled(ActionResult::ModifyCreatures {
                            targets: vec![target],
                            modifier,
                        });
                    }
                }
                Effect::CreateToken(token) => {
                    results.push_settled(ActionResult::CreateToken {
                        source: controller,
                        token: Box::new(token.clone()),
                    });
                }
                Effect::GainCounter(counter) => {
                    results.push_settled(ActionResult::AddCounters {
                        source,
                        target: source,
                        counter,
                    });
                }
                Effect::ControllerLosesLife(count) => {
                    results.push_settled(ActionResult::LoseLife {
                        target: source.controller(db),
                        count,
                    })
                }
                Effect::Mill(Mill { count, .. }) => {
                    results.push_settled(ActionResult::Mill { count, targets });
                }
                Effect::ReturnFromGraveyardToBattlefield(_) => {
                    results
                        .push_settled(ActionResult::ReturnFromGraveyardToBattlefield { targets });
                }
                Effect::ReturnFromGraveyardToLibrary(_) => {
                    results.push_settled(ActionResult::ReturnFromGraveyardToLibrary { targets });
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

                    results.push_choose_targets(ChooseTargets::new(
                        EffectOrAura::Effect(Effect::TutorLibrary(TutorLibrary {
                            restrictions,
                            destination,
                            reveal,
                        })),
                        valid_targets,
                    ));
                }
                Effect::CopyOfAnyCreatureNonTargeting => unreachable!(),
                Effect::CreateTokenCopy { modifiers } => {
                    let target = targets.into_iter().next().unwrap();
                    let target = target.id();
                    results.push_settled(ActionResult::CreateTokenCopyOf {
                        target: target.unwrap(),
                        modifiers,
                        controller: source.controller(db),
                    });
                }
                Effect::ReturnSelfToHand => {
                    source.move_to_hand(db);
                }
                Effect::RevealEachTopOfLibrary(reveal) => {
                    results.push_settled(ActionResult::RevealEachTopOfLibrary(source, reveal));
                }
                Effect::UntapThis => results.push_settled(ActionResult::Untap(source)),
                Effect::Cascade => results.push_settled(ActionResult::Cascade {
                    cascading: source.cost(db).cmc(),
                    player: controller,
                }),
                Effect::UntapTarget => {
                    let Ok(ActiveTarget::Battlefield { id }) = targets.into_iter().exactly_one()
                    else {
                        unreachable!()
                    };
                    results.push_settled(ActionResult::Untap(id));
                }
                Effect::TargetGainsCounters(counter) => {
                    let target = match targets.into_iter().exactly_one().unwrap() {
                        ActiveTarget::Battlefield { id } => id,
                        ActiveTarget::Graveyard { id } => id,
                        _ => unreachable!(),
                    };

                    results.push_settled(ActionResult::AddCounters {
                        source,
                        target,
                        counter,
                    })
                }
                Effect::Scry(count) => {
                    results.push_settled(ActionResult::Scry(source, count));
                }
                Effect::Discover(count) => results.push_settled(ActionResult::Discover {
                    count,
                    player: controller,
                }),
                Effect::ForEachManaOfSource(ForEachManaOfSource {
                    source: mana_source,
                    effect,
                }) => results.push_settled(ActionResult::ForEachManaOfSource {
                    card: source,
                    source: mana_source,
                    effect,
                }),
            }
        }

        if let Some(resolving_card) = resolving_card {
            if resolving_card.is_permanent(db) {
                results.push_settled(ActionResult::AddToBattlefield(
                    resolving_card,
                    targets.next().and_then(|targets| {
                        targets.into_iter().find_map(|target| match target {
                            ActiveTarget::Battlefield { id } => Some(id),
                            _ => None,
                        })
                    }),
                ));
            } else {
                results.push_settled(ActionResult::StackToGraveyard(resolving_card));
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

        results.push_settled(ActionResult::AddAbilityToStack {
            ability,
            source,
            targets: source.targets_for_ability(db, ability),
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
        let controller = source.controller(db);
        for effect in trigger.effects(db) {
            let effect = effect.into_effect(db, controller);
            targets.push(source.targets_for_effect(db, controller, &effect));
        }

        results.push_settled(ActionResult::AddTriggerToStack {
            trigger,
            source,
            targets,
        });

        results
    }

    pub fn move_card_to_stack_from_hand(
        db: &mut Database,
        card: CardId,
        paying_costs: bool,
    ) -> PendingResults {
        add_card_to_stack(card, db, CastFrom::Hand, paying_costs)
    }

    pub fn move_card_to_stack_from_exile(
        db: &mut Database,
        card: CardId,
        paying_costs: bool,
    ) -> PendingResults {
        add_card_to_stack(card, db, CastFrom::Exile, paying_costs)
    }
}

fn add_card_to_stack(
    card: CardId,
    db: &mut Database,
    from: CastFrom,
    paying_costs: bool,
) -> PendingResults {
    let mut results = PendingResults::new(Source::Card(card));

    match from {
        CastFrom::Hand => {
            card.cast_location::<cast_from::Hand>(db);
        }
        CastFrom::Exile => {
            card.cast_location::<cast_from::Exile>(db);
        }
    }
    card.apply_modifiers_layered(db);

    if card.wants_targets(db).into_iter().sum::<usize>() > 0 {
        results.add_card_to_stack(from);
        let controller = card.controller(db);
        if let Some(aura) = card.aura(db) {
            results.push_choose_targets(ChooseTargets::new(
                EffectOrAura::Aura(aura),
                card.targets_for_aura(db).unwrap(),
            ))
        }

        let effects = card.effects(db);
        if effects.len() == 1 {
            let effect = effects
                .into_iter()
                .exactly_one()
                .unwrap()
                .into_effect(db, controller);
            let valid_targets = card.targets_for_effect(db, controller, &effect);
            if valid_targets.len() < effect.needs_targets() {
                return PendingResults::default();
            }

            results.push_choose_targets(ChooseTargets::new(
                EffectOrAura::Effect(effect),
                valid_targets,
            ));
        } else {
            for effect in card.effects(db) {
                let effect = effect.into_effect(db, controller);
                let valid_targets = card.targets_for_effect(db, controller, &effect);
                results.push_choose_targets(ChooseTargets::new(
                    EffectOrAura::Effect(effect),
                    valid_targets,
                ));
            }
        }
    } else {
        results.push_settled(ActionResult::CastCard {
            card,
            targets: vec![],
            from,
            x_is: None,
        })
    }

    let cost = card.cost(db);
    if paying_costs {
        results.push_pay_costs(PayCost::SpendMana(SpendMana::new(cost.mana_cost.clone())));
    }

    results
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

        card1.move_to_stack(&mut db, Default::default(), None);

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
