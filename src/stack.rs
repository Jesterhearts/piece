use std::collections::HashMap;

use enumset::{enum_set, EnumSet};
use indexmap::IndexMap;

use crate::{
    battlefield::Battlefield,
    card::CastingModifier,
    controller::Controller,
    effects::{
        ActivatedAbilityEffect, BattlefieldModifier, EffectDuration, GainMana, ModifyBattlefield,
        SpellEffect,
    },
    in_play::{AllCards, AllModifiers, CardId, EffectsInPlay, ModifierInPlay},
    mana::Mana,
    player::PlayerRef,
    types::Type,
};

#[derive(Debug, PartialEq)]
pub enum StackResult {
    AddToBattlefield(CardId),
    ApplyToBattlefield {
        source: CardId,
        modifier: ModifierInPlay,
    },
    ExileTarget(CardId),
    ManifestTopOfLibrary(PlayerRef),
    ModifyCreatures {
        source: CardId,
        targets: Vec<CardId>,
        modifier: ModifierInPlay,
    },
    SpellCountered {
        id: usize,
    },
    RemoveSplitSecond,
    DrawCards {
        player: PlayerRef,
        count: usize,
    },
    GainMana {
        player: PlayerRef,
        mana: HashMap<Mana, usize>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ActiveTarget {
    Stack { id: usize },
    Battlefield { id: CardId },
}

#[derive(Debug, Clone)]
pub enum EntryType {
    Card(CardId),
    ActivatedAbility {
        source: CardId,
        effects: EffectsInPlay,
    },
}

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub ty: EntryType,
    pub active_target: Option<ActiveTarget>,
    pub mode: Option<usize>,
}

#[derive(Debug, Default)]
pub struct Stack {
    pub stack: IndexMap<usize, StackEntry>,
    next_id: usize,
    pub split_second: bool,
}

impl Stack {
    pub fn push_card(
        &mut self,
        cards: &AllCards,
        card: CardId,
        target: Option<ActiveTarget>,
        mode: Option<usize>,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        if cards[card]
            .card
            .casting_modifiers
            .contains(&CastingModifier::SplitSecond)
        {
            self.split_second = true;
        }

        self.stack.insert(
            id,
            StackEntry {
                ty: EntryType::Card(card),
                active_target: target,
                mode,
            },
        );

        id
    }
    pub fn push_activated_ability(
        &mut self,
        source: CardId,
        effects: EffectsInPlay,
        target: Option<ActiveTarget>,
    ) {
        let id = self.next_id;
        self.next_id += 1;
        self.stack.insert(
            id,
            StackEntry {
                ty: EntryType::ActivatedAbility { source, effects },
                active_target: target,
                mode: None,
            },
        );
    }

    pub fn target_nth(&self, nth: usize) -> Option<ActiveTarget> {
        self.stack
            .keys()
            .copied()
            .nth(nth)
            .map(|id| ActiveTarget::Stack { id })
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    #[must_use]
    pub fn resolve_1(&mut self, cards: &AllCards, battlefield: &Battlefield) -> Vec<StackResult> {
        let Some((_, next)) = self.stack.pop() else {
            return vec![];
        };

        let mut result = vec![];

        match next.ty {
            EntryType::Card(card) => {
                let resolving_card = &cards[card];

                if resolving_card
                    .card
                    .casting_modifiers
                    .contains(&CastingModifier::SplitSecond)
                {
                    result.push(StackResult::RemoveSplitSecond);
                }

                for effect in resolving_card.card.effects.iter() {
                    match effect {
                        SpellEffect::CounterSpell { target } => {
                            match next.active_target {
                                Some(active_target) => {
                                    match active_target {
                                        ActiveTarget::Stack { id } => {
                                            let Some(maybe_target) = &self.stack.get(&id) else {
                                                // Spell has left the stack already
                                                return vec![];
                                            };

                                            match &maybe_target.ty {
                                                EntryType::Card(maybe_target) => {
                                                    let maybe_target = &cards[*maybe_target];
                                                    if !maybe_target.card.can_be_countered(
                                                        cards,
                                                        battlefield,
                                                        &resolving_card.controller.borrow(),
                                                        &maybe_target.controller.borrow(),
                                                        target,
                                                    ) {
                                                        // Spell is no longer a valid target.
                                                        return vec![];
                                                    }
                                                }
                                                EntryType::ActivatedAbility { .. } => {
                                                    // Vanilla counterspells can't counter activated abilities.
                                                    return vec![];
                                                }
                                            }

                                            // If we reach here, we know the spell can be countered.
                                            result.push(StackResult::SpellCountered { id });
                                        }
                                        ActiveTarget::Battlefield { .. } => {
                                            // Cards on the battlefield aren't valid targets of counterspells
                                            return vec![];
                                        }
                                    }
                                }
                                None => {
                                    // Spell fizzles due to lack of target.
                                    return vec![];
                                }
                            };
                        }
                        SpellEffect::GainMana { mana } => {
                            if !gain_mana(mana, next.mode, &mut result, cards, card) {
                                return vec![];
                            }
                        }
                        SpellEffect::BattlefieldModifier(modifier) => {
                            result.push(StackResult::ApplyToBattlefield {
                                source: card,
                                modifier: ModifierInPlay {
                                    modifier: modifier.clone(),
                                    controller: cards[card].controller.clone(),
                                    modifying: Default::default(),
                                },
                            });
                        }
                        SpellEffect::ControllerDrawCards(count) => {
                            result.push(StackResult::DrawCards {
                                player: cards[card].controller.clone(),
                                count: *count,
                            });
                        }
                        SpellEffect::AddPowerToughnessToTarget(modifier) => {
                            if !add_power_toughness(
                                next.active_target,
                                battlefield,
                                cards,
                                card,
                                &mut result,
                                modifier,
                            ) {
                                return vec![];
                            }
                        }
                        SpellEffect::ModifyCreature(modifier) => {
                            if !modify_creature(
                                next.active_target,
                                battlefield,
                                cards,
                                card,
                                &mut result,
                                modifier,
                            ) {
                                return vec![];
                            };
                        }
                        SpellEffect::ExileTargetCreature => {
                            match next.active_target {
                                Some(active_target) => match active_target {
                                    ActiveTarget::Stack { .. } => return vec![],
                                    ActiveTarget::Battlefield { id } => {
                                        let Some(target) = battlefield.get(id) else {
                                            // Permanent no longer on battlefield.
                                            return vec![];
                                        };

                                        if !cards[target].card.can_be_targeted(
                                            &cards[card].controller.borrow(),
                                            &cards[target].controller.borrow(),
                                        ) {
                                            // Card is no longer a valid target.
                                            return vec![];
                                        }

                                        if !cards[target].card.types_intersect(&[Type::Creature]) {
                                            // Target isn't a creature
                                            return vec![];
                                        }

                                        result.push(StackResult::ExileTarget(target));
                                    }
                                },
                                None => {
                                    return vec![];
                                }
                            };
                        }
                        SpellEffect::ExileTargetCreatureManifestTopOfLibrary => {
                            match next.active_target {
                                Some(active_target) => match active_target {
                                    ActiveTarget::Stack { .. } => return vec![],
                                    ActiveTarget::Battlefield { id } => {
                                        let Some(target) = battlefield.get(id) else {
                                            // Permanent no longer on battlefield.
                                            return vec![];
                                        };

                                        if !cards[target].card.can_be_targeted(
                                            &cards[card].controller.borrow(),
                                            &cards[target].controller.borrow(),
                                        ) {
                                            // Card is no longer a valid target.
                                            return vec![];
                                        }

                                        if !cards[target].card.types_intersect(&[Type::Creature]) {
                                            // Target isn't a creature
                                            return vec![];
                                        }

                                        result.push(StackResult::ExileTarget(target));
                                        result.push(StackResult::ManifestTopOfLibrary(
                                            cards[target].controller.clone(),
                                        ));
                                    }
                                },
                                None => return vec![],
                            };
                        }
                    }
                }

                if resolving_card.card.is_permanent() {
                    result.push(StackResult::AddToBattlefield(card));
                }

                result
            }
            EntryType::ActivatedAbility { source, effects } => {
                for effect in effects.effects.into_iter() {
                    match effect {
                        ActivatedAbilityEffect::CounterSpell { target: _ } => todo!(),
                        ActivatedAbilityEffect::GainMana { mana } => {
                            if !gain_mana(&mana, next.mode, &mut result, cards, source) {
                                return vec![];
                            }
                        }
                        ActivatedAbilityEffect::BattlefieldModifier(modifier) => {
                            result.push(StackResult::ApplyToBattlefield {
                                source,
                                modifier: ModifierInPlay {
                                    modifier,
                                    controller: effects.controller.clone(),
                                    modifying: Default::default(),
                                },
                            });
                        }
                        ActivatedAbilityEffect::ControllerDrawCards(count) => {
                            result.push(StackResult::DrawCards {
                                player: effects.controller.clone(),
                                count,
                            });
                        }
                        ActivatedAbilityEffect::Equip(modifiers) => {
                            let Some(target) = next.active_target else {
                                // Effect fizzles due to lack of target.
                                return vec![];
                            };

                            match target {
                                ActiveTarget::Stack { .. } => {
                                    // Can't equip things on the stack
                                    return vec![];
                                }
                                ActiveTarget::Battlefield { id } => {
                                    for modifier in modifiers {
                                        let card = &cards[id];
                                        if !card.card.can_be_targeted(
                                            &cards[effects.source].controller.borrow(),
                                            &card.controller.borrow(),
                                        ) {
                                            // Card is not a valid target, spell fizzles.
                                            return vec![];
                                        }

                                        result.push(StackResult::ModifyCreatures {
                                            source: effects.source,
                                            targets: vec![id],
                                            modifier: ModifierInPlay {
                                                modifier: BattlefieldModifier {
                                                    modifier,
                                                    controller: Controller::You,
                                                    duration: EffectDuration::UntilUnattached,
                                                    restrictions: enum_set!(),
                                                },
                                                controller: card.controller.clone(),
                                                modifying: vec![],
                                            },
                                        });
                                    }
                                }
                            }
                        }
                        ActivatedAbilityEffect::AddPowerToughnessToTarget(modifier) => {
                            if !add_power_toughness(
                                next.active_target,
                                battlefield,
                                cards,
                                source,
                                &mut result,
                                &modifier,
                            ) {
                                return vec![];
                            }
                        }
                    }
                }
                result
            }
        }
    }

    pub fn apply_results(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        battlefield: &mut Battlefield,
        results: Vec<StackResult>,
    ) {
        for result in results {
            match result {
                StackResult::AddToBattlefield(card) => {
                    let results = battlefield.add(cards, modifiers, card, vec![]);
                    battlefield.apply_action_results(cards, modifiers, self, results);
                }
                StackResult::ApplyToBattlefield { source, modifier } => {
                    let modifier_id = modifiers.add_modifier(modifier);
                    battlefield.apply_modifier(cards, modifiers, source, modifier_id)
                }
                StackResult::ExileTarget(target) => {
                    battlefield.exile(cards, modifiers, self, target);
                }
                StackResult::ManifestTopOfLibrary(player) => {
                    player
                        .borrow_mut()
                        .manifest(cards, modifiers, battlefield, self);
                }
                StackResult::ModifyCreatures {
                    source,
                    targets,
                    modifier,
                } => {
                    let modifier_id = modifiers.add_modifier(modifier);
                    battlefield.apply_modifier_to_targets(
                        cards,
                        modifiers,
                        source,
                        modifier_id,
                        &targets,
                    );
                }
                StackResult::SpellCountered { id } => {
                    let removed = self.stack.remove(&id);
                    assert!(removed.is_some());
                }
                StackResult::RemoveSplitSecond => {
                    self.split_second = false;
                }
                StackResult::DrawCards { player, count } => {
                    player.borrow_mut().draw(count);
                }
                StackResult::GainMana { player, mana } => {
                    for (mana, count) in mana {
                        for _ in 0..count {
                            player.borrow_mut().mana_pool.apply(mana);
                        }
                    }
                }
            }
        }
    }
}

fn add_power_toughness(
    active_target: Option<ActiveTarget>,
    battlefield: &Battlefield,
    cards: &AllCards,
    card: CardId,
    result: &mut Vec<StackResult>,
    modifier: &crate::effects::AddPowerToughness,
) -> bool {
    match active_target {
        Some(active_target) => {
            match active_target {
                ActiveTarget::Stack { .. } => {
                    // Stack is not a valid target.
                    return false;
                }
                ActiveTarget::Battlefield { id } => {
                    let Some(target) = battlefield.get(id) else {
                        // Permanent no longer on battlefield.
                        return false;
                    };

                    if !cards[target].card.can_be_targeted(
                        &cards[card].controller.borrow(),
                        &cards[target].controller.borrow(),
                    ) {
                        // Card is no longer a valid target.
                        return false;
                    }

                    result.push(StackResult::ModifyCreatures {
                        source: card,
                        targets: vec![target],
                        modifier: ModifierInPlay {
                            modifier: BattlefieldModifier {
                                modifier: ModifyBattlefield::AddPowerToughness(modifier.clone()),
                                controller: Controller::Any,
                                duration: EffectDuration::UntilEndOfTurn,
                                restrictions: enum_set!(),
                            },
                            controller: cards[card].controller.clone(),
                            modifying: Default::default(),
                        },
                    });
                }
            };
        }
        None => {
            // Spell fizzles due to lack of target.
            return false;
        }
    }

    true
}

fn modify_creature(
    active_target: Option<ActiveTarget>,
    battlefield: &Battlefield,
    cards: &AllCards,
    card: CardId,
    result: &mut Vec<StackResult>,
    modifier: &BattlefieldModifier,
) -> bool {
    match active_target {
        Some(active_target) => {
            match active_target {
                ActiveTarget::Stack { .. } => {
                    // Stack is not a valid target.
                    return false;
                }
                ActiveTarget::Battlefield { id } => {
                    let Some(target) = battlefield.get(id) else {
                        // Permanent no longer on battlefield.
                        return false;
                    };

                    if !cards[target].card.can_be_targeted(
                        &cards[card].controller.borrow(),
                        &cards[target].controller.borrow(),
                    ) {
                        // Card is no longer a valid target.
                        return false;
                    }

                    result.push(StackResult::ModifyCreatures {
                        source: card,
                        targets: vec![target],
                        modifier: ModifierInPlay {
                            modifier: modifier.clone(),
                            controller: cards[card].controller.clone(),
                            modifying: Default::default(),
                        },
                    });
                }
            };
        }
        None => {
            // Spell fizzles due to lack of target.
            return false;
        }
    }
    true
}

fn gain_mana(
    mana: &GainMana,
    mode: Option<usize>,
    result: &mut Vec<StackResult>,
    cards: &AllCards,
    card: CardId,
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
        player: cards[card].controller.clone(),
        mana: manas,
    });

    true
}

#[cfg(test)]
mod tests {
    use crate::{
        battlefield::Battlefield,
        deck::Deck,
        in_play::AllCards,
        load_cards,
        player::Player,
        stack::{Stack, StackResult},
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::empty());
        let mut all_cards = AllCards::default();
        let battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let creature = all_cards.add(&cards, player.clone(), "Allosaurus Shepherd");

        stack.push_card(&all_cards, creature, None, None);
        let result = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(result, [StackResult::AddToBattlefield(creature)]);

        assert!(stack.is_empty());

        Ok(())
    }
}
