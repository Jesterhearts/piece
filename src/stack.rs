use std::collections::{HashMap, HashSet};

use bevy_ecs::{component::Component, entity::Entity, query::With};
use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    card::SplitSecond,
    controller::ControllerRestriction,
    effects::{BattlefieldModifier, Counter, Effect, EffectDuration, GainMana, Token},
    in_play::{
        AbilityId, CardId, CounterId, Database, InStack, ModifierId, OnBattlefield, TriggerId,
    },
    mana::Mana,
    player::{AllPlayers, Controller},
    targets::{Restriction, SpellTarget},
    types::Type,
};

#[derive(Debug, PartialEq)]
pub enum StackResult {
    AddToBattlefield(CardId),
    StackToGraveyard(CardId),
    ApplyToBattlefield(ModifierId),
    ApplyModifierToTarget {
        modifier: ModifierId,
        target: CardId,
    },
    ExileTarget(CardId),
    DamageTarget {
        quantity: usize,
        target: CardId,
    },
    ManifestTopOfLibrary(Controller),
    ModifyCreatures {
        targets: Vec<CardId>,
        modifier: ModifierId,
    },
    SpellCountered {
        id: Entry,
    },
    DrawCards {
        player: Controller,
        count: usize,
    },
    GainMana {
        player: Controller,
        mana: HashMap<Mana, usize>,
    },
    CreateToken {
        source: CardId,
        token: Token,
    },
    AddCounters {
        target: CardId,
        counter: Counter,
        count: usize,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Component)]
pub struct Targets(pub HashSet<ActiveTarget>);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ActiveTarget {
    Stack { id: InStack },
    Battlefield { id: CardId },
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Entry {
    Card(CardId),
    Ability(AbilityId),
    Trigger(TriggerId),
}

#[derive(Debug, Clone, Copy, Deref, DerefMut, Component)]
pub struct Mode(pub usize);

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub ty: Entry,
    pub targets: HashSet<ActiveTarget>,
    pub mode: Option<usize>,
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
            .chain(db.abilities.query::<&InStack>().iter(&db.abilities))
            .chain(db.triggers.query::<&InStack>().iter(&db.triggers))
            .sorted()
            .collect_vec()[nth];

        ActiveTarget::Stack { id: *nth }
    }

    pub fn in_stack(db: &mut Database) -> HashMap<InStack, Entry> {
        db.cards
            .query::<(&InStack, Entity)>()
            .iter(&db.cards)
            .map(|(seq, entity)| (*seq, Entry::Card(entity.into())))
            .chain(
                db.abilities
                    .query::<(&InStack, Entity)>()
                    .iter(&db.abilities)
                    .map(|(seq, entity)| (*seq, Entry::Ability(entity.into()))),
            )
            .chain(
                db.triggers
                    .query::<(&InStack, Entity)>()
                    .iter(&db.triggers)
                    .map(|(seq, entity)| (*seq, Entry::Trigger(entity.into()))),
            )
            .sorted_by_key(|(seq, _)| *seq)
            .collect()
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
                    .query::<(&InStack, Entity, &Targets, Option<&Mode>)>()
                    .iter(&db.abilities)
                    .map(|(seq, entity, targets, mode)| {
                        (
                            *seq,
                            StackEntry {
                                ty: Entry::Ability(entity.into()),
                                targets: targets.0.clone(),
                                mode: mode.map(|mode| mode.0),
                            },
                        )
                    }),
            )
            .chain(
                db.triggers
                    .query::<(&InStack, Entity, &Targets, Option<&Mode>)>()
                    .iter(&db.triggers)
                    .map(|(seq, entity, targets, mode)| {
                        (
                            *seq,
                            StackEntry {
                                ty: Entry::Trigger(entity.into()),
                                targets: targets.0.clone(),
                                mode: mode.map(|mode| mode.0),
                            },
                        )
                    }),
            )
            .sorted_by_key(|(seq, _)| *seq)
            .last()
            .map(|(_, entry)| entry)
    }

    pub fn is_empty(db: &mut Database) -> bool {
        db.cards
            .query::<&InStack>()
            .iter(&db.cards)
            .chain(db.abilities.query::<&InStack>().iter(&db.abilities))
            .chain(db.triggers.query::<&InStack>().iter(&db.triggers))
            .next()
            .is_none()
    }

    pub fn resolve_1(db: &mut Database) -> Vec<StackResult> {
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
            Entry::Ability(ability) => (
                ability.apply_to_self(db),
                ability.effects(db),
                ability.controller(db),
                None,
                ability.source(db),
            ),
            Entry::Trigger(trigger) => {
                let listener = trigger.listener(db);
                (
                    false,
                    trigger.effects(db),
                    listener.controller(db),
                    None,
                    listener,
                )
            }
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
                            return vec![StackResult::StackToGraveyard(resolving_card)];
                        } else {
                            return vec![];
                        }
                    }
                }
                Effect::GainMana { mana } => {
                    if !gain_mana(controller, &mana, next.mode, &mut results) {
                        if let Some(resolving_card) = resolving_card {
                            return vec![StackResult::StackToGraveyard(resolving_card)];
                        } else {
                            return vec![];
                        }
                    }
                }
                Effect::BattlefieldModifier(modifier) => {
                    if apply_to_self {
                        let modifier = ModifierId::upload_temporary_modifier(db, source, &modifier);
                        results.push(StackResult::ApplyModifierToTarget {
                            modifier,
                            target: source,
                        });
                    } else {
                        results.push(StackResult::ApplyToBattlefield(
                            ModifierId::upload_temporary_modifier(db, source, &modifier),
                        ));
                    }
                }
                Effect::ControllerDrawCards(count) => {
                    results.push(StackResult::DrawCards {
                        player: controller,
                        count,
                    });
                }
                Effect::ModifyCreature(modifier) => {
                    let modifier = ModifierId::upload_temporary_modifier(db, source, &modifier);

                    let mut targets = vec![];
                    for target in next.targets.iter() {
                        match target {
                            ActiveTarget::Stack { .. } => {
                                // Stack is not a valid target.
                                if let Some(resolving_card) = resolving_card {
                                    return vec![StackResult::StackToGraveyard(resolving_card)];
                                } else {
                                    return vec![];
                                }
                            }
                            ActiveTarget::Battlefield { id } => {
                                targets.push(id);
                            }
                        }
                    }

                    for target in targets {
                        results.push(StackResult::ApplyModifierToTarget {
                            modifier,
                            target: *target,
                        });
                    }
                }
                Effect::ExileTargetCreature => {
                    for target in next.targets.iter() {
                        match target {
                            ActiveTarget::Stack { .. } => {
                                if let Some(resolving_card) = resolving_card {
                                    return vec![StackResult::StackToGraveyard(resolving_card)];
                                } else {
                                    return vec![];
                                }
                            }
                            ActiveTarget::Battlefield { id } => {
                                if !id.is_in_location::<OnBattlefield>(db) {
                                    // Permanent no longer on battlefield.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![StackResult::StackToGraveyard(resolving_card)];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.can_be_targeted(db, controller) {
                                    // Card is no longer a valid target.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![StackResult::StackToGraveyard(resolving_card)];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.types_intersect(db, &HashSet::from([Type::Creature])) {
                                    // Target isn't a creature

                                    if let Some(resolving_card) = resolving_card {
                                        return vec![StackResult::StackToGraveyard(resolving_card)];
                                    } else {
                                        return vec![];
                                    }
                                }

                                results.push(StackResult::ExileTarget(*id));
                            }
                        }
                    }
                }
                Effect::ExileTargetCreatureManifestTopOfLibrary => {
                    for target in next.targets.iter() {
                        match target {
                            ActiveTarget::Stack { .. } => {
                                if let Some(resolving_card) = resolving_card {
                                    return vec![StackResult::StackToGraveyard(resolving_card)];
                                } else {
                                    return vec![];
                                }
                            }
                            ActiveTarget::Battlefield { id } => {
                                if !id.is_in_location::<OnBattlefield>(db) {
                                    // Permanent no longer on battlefield.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![StackResult::StackToGraveyard(resolving_card)];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.can_be_targeted(db, controller) {
                                    // Card is no longer a valid target.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![StackResult::StackToGraveyard(resolving_card)];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.types_intersect(db, &HashSet::from([Type::Creature])) {
                                    // Target isn't a creature
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![StackResult::StackToGraveyard(resolving_card)];
                                    } else {
                                        return vec![];
                                    }
                                }

                                results.push(StackResult::ExileTarget(*id));
                                results.push(StackResult::ManifestTopOfLibrary(id.controller(db)));
                            }
                        }
                    }
                }
                Effect::DealDamage(dmg) => {
                    for target in next.targets.iter() {
                        match target {
                            ActiveTarget::Stack { .. } => {
                                if let Some(resolving_card) = resolving_card {
                                    return vec![StackResult::StackToGraveyard(resolving_card)];
                                } else {
                                    return vec![];
                                }
                            }
                            ActiveTarget::Battlefield { id } => {
                                if !id.is_in_location::<OnBattlefield>(db) {
                                    // Permanent no longer on battlefield.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![StackResult::StackToGraveyard(resolving_card)];
                                    } else {
                                        return vec![];
                                    }
                                }

                                if !id.can_be_targeted(db, controller) {
                                    // Card is no longer a valid target.
                                    if let Some(resolving_card) = resolving_card {
                                        return vec![StackResult::StackToGraveyard(resolving_card)];
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
                                        return vec![StackResult::StackToGraveyard(resolving_card)];
                                    } else {
                                        return vec![];
                                    }
                                }

                                results.push(StackResult::DamageTarget {
                                    quantity: dmg.quantity,
                                    target: *id,
                                });
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

                    match next.targets.iter().next().unwrap() {
                        ActiveTarget::Stack { .. } => {
                            // Can't equip things on the stack
                            return vec![];
                        }
                        ActiveTarget::Battlefield { id } => {
                            if !id.can_be_targeted(db, controller) {
                                // Card is not a valid target, spell fizzles.
                                return vec![];
                            }

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

                                results.push(StackResult::ModifyCreatures {
                                    targets: vec![*id],
                                    modifier,
                                });
                            }
                        }
                    }
                }
                Effect::CreateToken(token) => {
                    results.push(StackResult::CreateToken {
                        source,
                        token: token.clone(),
                    });
                }
                Effect::GainCounter(counter) => {
                    results.push(StackResult::AddCounters {
                        target: source,
                        counter,
                        count: 1,
                    });
                }
            }
        }

        if let Some(resolving_card) = resolving_card {
            if resolving_card.is_permanent(db) {
                results.push(StackResult::AddToBattlefield(resolving_card));
            } else {
                results.push(StackResult::StackToGraveyard(resolving_card));
            }
        }

        results
    }

    pub fn apply_results(
        db: &mut Database,
        all_players: &mut AllPlayers,
        results: Vec<StackResult>,
    ) -> Vec<UnresolvedActionResult> {
        let mut pending = vec![];

        for result in results {
            match result {
                StackResult::AddToBattlefield(card) => {
                    pending.extend(Battlefield::add_from_stack(db, card, vec![]));
                }
                StackResult::ApplyToBattlefield(modifier) => {
                    modifier.activate(db);
                }
                StackResult::ExileTarget(target) => {
                    pending.extend(Battlefield::exile(db, target));
                }
                StackResult::ManifestTopOfLibrary(player) => {
                    pending.extend(all_players[player].manifest(db));
                }
                StackResult::ModifyCreatures { targets, modifier } => {
                    for target in targets {
                        target.apply_modifier(db, modifier);
                    }
                }
                StackResult::SpellCountered { id } => match id {
                    Entry::Card(card) => {
                        pending.extend(Battlefield::stack_to_graveyard(db, card));
                    }
                    Entry::Ability(_) | Entry::Trigger(_) => unreachable!(),
                },
                StackResult::DrawCards { player, count } => {
                    all_players[player].draw(db, count);
                }
                StackResult::GainMana { player, mana } => {
                    for (mana, count) in mana {
                        for _ in 0..count {
                            all_players[player].mana_pool.apply(mana);
                        }
                    }
                }
                StackResult::StackToGraveyard(card) => {
                    pending.extend(Battlefield::stack_to_graveyard(db, card));
                }
                StackResult::ApplyModifierToTarget { modifier, target } => {
                    target.apply_modifier(db, modifier);
                }
                StackResult::CreateToken { source, token } => {
                    let controller = source.controller(db);
                    let id = CardId::upload_token(db, controller.into(), token);
                    pending.extend(Battlefield::add_from_stack(db, id, vec![]));
                }
                StackResult::DamageTarget { quantity, target } => {
                    target.mark_damage(db, quantity);
                }
                StackResult::AddCounters {
                    target,
                    counter,
                    count,
                } => {
                    CounterId::add_counters(db, target, counter, count);
                }
            }
        }

        pending
    }
}

fn counter_spell(
    db: &mut Database,
    in_stack: &HashMap<InStack, Entry>,
    controller: Controller,
    targets: &HashSet<ActiveTarget>,
    restrictions: &SpellTarget,
    result: &mut Vec<StackResult>,
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
                    Entry::Ability(_) => {
                        // Vanilla counterspells can't counter activated abilities.
                        return false;
                    }
                    Entry::Trigger(_) => {
                        // Vanilla counterspells can't counter triggered abilities.
                        return false;
                    }
                }

                // If we reach here, we know the spell can be countered.
                result.push(StackResult::SpellCountered { id: *maybe_target });
            }
            ActiveTarget::Battlefield { .. } => {
                // Cards on the battlefield aren't valid targets of counterspells
                return false;
            }
        }
    }

    true
}

fn gain_mana(
    controller: Controller,
    mana: &GainMana,
    mode: Option<usize>,
    result: &mut Vec<StackResult>,
) -> bool {
    let mut manas = HashMap::default();
    match mana {
        GainMana::Specific { gains } => {
            for gain in gains.iter() {
                *manas.entry(*gain).or_default() += 1;
            }
        }
        GainMana::Choice { choices } => {
            let Some(mode) = mode else {
                // No mode selected for modal ability.
                return false;
            };

            for gain in choices[mode].iter() {
                *manas.entry(*gain).or_default() += 1;
            }
        }
    };

    result.push(StackResult::GainMana {
        player: controller,
        mana: manas,
    });

    true
}

#[cfg(test)]
mod tests {
    use crate::{
        in_play::{CardId, Database},
        load_cards,
        player::AllPlayers,
        stack::{Stack, StackResult},
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let mut db = Database::default();
        let mut all_players = AllPlayers::default();
        let player = all_players.new_player();
        let card1 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");

        card1.move_to_stack(&mut db, Default::default());

        let results = Stack::resolve_1(&mut db);

        assert_eq!(results, [StackResult::AddToBattlefield(card1)]);
        Stack::apply_results(&mut db, &mut all_players, results);

        assert!(Stack::is_empty(&mut db));

        Ok(())
    }
}
