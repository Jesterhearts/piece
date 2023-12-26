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
        choose_targets::ChooseTargets,
        pay_costs::SacrificePermanent,
        pay_costs::SpendMana,
        pay_costs::TapPermanent,
        pay_costs::{ExileCards, ExilePermanentsCmcX},
        pay_costs::{ExileCardsSharingType, PayCost},
        ActionResult, PendingResults, Source, TargetSource,
    },
    card::keyword::SplitSecond,
    cost::AdditionalCost,
    in_play::{
        cast_from, AbilityId, CardId, CastFrom, Database, InStack, TriggerId, TriggerInStack,
    },
    player::{mana_pool::SpendReason, AllPlayers, Owner},
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
    pub fn display(&self, db: &mut Database, all_players: &AllPlayers) -> String {
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
            ActiveTarget::Player { id } => all_players[*id].name.clone(),
        }
    }

    pub fn id(&self) -> Option<CardId> {
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

impl Entry {
    pub fn source(&self) -> CardId {
        match self {
            Entry::Card(card_source)
            | Entry::Ability { card_source, .. }
            | Entry::Trigger { card_source, .. } => *card_source,
        }
    }
}

#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct Modes(pub Vec<usize>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackEntry {
    pub ty: Entry,
    pub targets: Vec<Vec<ActiveTarget>>,
    pub mode: Option<Vec<usize>>,
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
            .query_filtered::<(&InStack, Entity, &Targets, Option<&Modes>), Without<Settled>>()
            .iter(&db.cards)
            .map(|(seq, entity, targets, mode)| {
                (
                    *seq,
                    StackEntry {
                        ty: Entry::Card(entity.into()),
                        targets: targets.0.clone(),
                        mode: mode.map(|mode| mode.0.clone()),
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
                        Option<&Modes>,
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
                        mode: mode.map(|mode| mode.0.clone()),
                            },
                        )
                    }),
            )
            .chain(
                db.triggers
                    .query_filtered::<(&TriggerInStack, Entity, &Targets, Option<&Modes>), Without<Settled>>()
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
                        mode: mode.map(|mode| mode.0.clone()),
                            },
                        )
                    }),
            )
            .sorted_by_key(|(seq, _)| -*seq)
            .collect_vec()
    }

    pub fn entries(db: &mut Database) -> Vec<(InStack, StackEntry)> {
        db.cards
            .query::<(&InStack, Entity, &Targets, Option<&Modes>)>()
            .iter(&db.cards)
            .map(|(seq, entity, targets, mode)| {
                (
                    *seq,
                    StackEntry {
                        ty: Entry::Card(entity.into()),
                        targets: targets.0.clone(),
                        mode: mode.map(|mode| mode.0.clone()),
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
                        Option<&Modes>,
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
                                mode: mode.map(|mode| mode.0.clone()),
                            },
                        )
                    }),
            )
            .chain(
                db.triggers
                    .query::<(&TriggerInStack, Entity, &Targets, Option<&Modes>)>()
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
                                mode: mode.map(|mode| mode.0.clone()),
                            },
                        )
                    }),
            )
            .sorted_by_key(|(seq, _)| -*seq)
            .collect_vec()
    }

    fn pop(db: &mut Database) -> Option<StackEntry> {
        db.cards
            .query::<(&InStack, Entity, &Targets, Option<&Modes>)>()
            .iter(&db.cards)
            .map(|(seq, entity, targets, mode)| {
                (
                    *seq,
                    StackEntry {
                        ty: Entry::Card(entity.into()),
                        targets: targets.0.clone(),
                        mode: mode.map(|mode| mode.0.clone()),
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
                        Option<&Modes>,
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
                                mode: mode.map(|mode| mode.0.clone()),
                            },
                        )
                    }),
            )
            .chain(
                db.triggers
                    .query::<(Entity, &TriggerInStack, &Targets, Option<&Modes>)>()
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
                                mode: mode.map(|mode| mode.0.clone()),
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
            Entry::Card(card) => {
                let effects = if let Some(modes) = card.modes(db) {
                    debug!("Modes: {:?}", modes);
                    modes
                        .0
                        .into_iter()
                        .nth(next.mode.unwrap().into_iter().exactly_one().unwrap())
                        .unwrap()
                        .effects
                } else {
                    card.effects(db)
                };

                (false, effects, card.controller(db), Some(card), card)
            }
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

        let mut results = PendingResults::default();
        results.apply_in_stages();

        let mut targets = next.targets.into_iter();
        for (effect, targets) in effects
            .into_iter()
            .zip((&mut targets).chain(std::iter::repeat(vec![])))
        {
            let effect = effect.into_effect(db, controller);
            if targets.len() != effect.needs_targets() && effect.needs_targets() != 0 {
                let valid_targets =
                    effect.valid_targets(db, source, controller, &HashSet::default());
                results.push_choose_targets(ChooseTargets::new(
                    TargetSource::Effect(effect),
                    valid_targets,
                    source,
                ));
                continue;
            }

            if effect.wants_targets() > 0 {
                let valid_targets = effect
                    .valid_targets(db, source, controller, &HashSet::default())
                    .into_iter()
                    .collect::<HashSet<_>>();
                if !targets.iter().all(|target| valid_targets.contains(target)) {
                    if let Some(resolving_card) = resolving_card {
                        let mut results = PendingResults::default();
                        results.push_settled(ActionResult::StackToGraveyard(resolving_card));
                        return results;
                    } else {
                        return PendingResults::default();
                    }
                }
            }

            effect.push_behavior_with_targets(
                db,
                targets,
                apply_to_self,
                source,
                controller,
                &mut results,
            );
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
            targets: source.targets_for_ability(db, ability, &HashSet::default()),
            x_is: None,
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
            targets.push(effect.valid_targets(db, source, controller, &HashSet::default()));
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
    let mut results = PendingResults::default();

    match from {
        CastFrom::Hand => {
            card.cast_location::<cast_from::Hand>(db);
        }
        CastFrom::Exile => {
            card.cast_location::<cast_from::Exile>(db);
        }
    }
    card.apply_modifiers_layered(db);

    if card.has_modes(db) {
        results.push_choose_mode(Source::Card(card));
    }

    results.add_card_to_stack(card, from);
    if card.wants_targets(db).into_iter().sum::<usize>() > 0 {
        let controller = card.controller(db);
        if let Some(aura) = card.aura(db) {
            results.push_choose_targets(ChooseTargets::new(
                TargetSource::Aura(aura),
                card.targets_for_aura(db).unwrap(),
                card,
            ))
        }

        let effects = card.effects(db);
        if effects.len() == 1 {
            let effect = effects
                .into_iter()
                .exactly_one()
                .unwrap()
                .into_effect(db, controller);
            let valid_targets = effect.valid_targets(db, card, controller, &HashSet::default());
            if valid_targets.len() < effect.needs_targets() {
                return PendingResults::default();
            }

            results.push_choose_targets(ChooseTargets::new(
                TargetSource::Effect(effect),
                valid_targets,
                card,
            ));
        } else {
            for effect in card.effects(db) {
                let effect = effect.into_effect(db, controller);
                let valid_targets = effect.valid_targets(db, card, controller, &HashSet::default());
                results.push_choose_targets(ChooseTargets::new(
                    TargetSource::Effect(effect),
                    valid_targets,
                    card,
                ));
            }
        }
    }

    let cost = card.cost(db);
    if paying_costs {
        results.push_pay_costs(PayCost::SpendMana(SpendMana::new(
            cost.mana_cost.clone(),
            card,
            SpendReason::Casting(card),
        )));
    }
    for cost in cost.additional_cost.iter() {
        match cost {
            AdditionalCost::SacrificeSource => unreachable!(),
            AdditionalCost::PayLife(_) => todo!(),
            AdditionalCost::SacrificePermanent(restrictions) => {
                results.push_pay_costs(PayCost::SacrificePermanent(SacrificePermanent::new(
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::TapPermanent(restrictions) => {
                results.push_pay_costs(PayCost::TapPermanent(TapPermanent::new(
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::ExileCardsCmcX(restrictions) => {
                results.push_pay_costs(PayCost::ExilePermanentsCmcX(ExilePermanentsCmcX::new(
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::ExileCard { restrictions } => {
                results.push_pay_costs(PayCost::ExileCards(ExileCards::new(
                    None,
                    1,
                    1,
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::ExileXOrMoreCards {
                minimum,
                restrictions,
            } => {
                results.push_pay_costs(PayCost::ExileCards(ExileCards::new(
                    None,
                    *minimum,
                    usize::MAX,
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::ExileSharingCardType { count } => {
                results.push_pay_costs(PayCost::ExileCardsSharingType(ExileCardsSharingType::new(
                    None, card, *count,
                )));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use crate::{
        battlefield::{Battlefield, ResolutionResult},
        in_play::{CardId, Database},
        load_cards,
        player::AllPlayers,
        stack::Stack,
        turns::Turn,
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let mut db = Database::default();
        let mut all_players = AllPlayers::default();
        let player = all_players.new_player("Player".to_string(), 20);
        let turn = Turn::new(&all_players);
        let card1 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");

        card1.move_to_stack(&mut db, Default::default(), None, vec![]);

        let mut results = Stack::resolve_1(&mut db);

        let result = results.resolve(&mut db, &mut all_players, &turn, None);
        assert_eq!(result, ResolutionResult::Complete);

        assert!(Stack::is_empty(&mut db));
        assert_eq!(Battlefield::creatures(&mut db), [card1]);

        Ok(())
    }
}
