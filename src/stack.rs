use std::collections::BTreeMap;

use crate::{
    battlefield::Battlefield,
    card::{Controller, Effect, PlayedCard, Target},
};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ActiveTarget {
    Stack { id: usize },
}

#[derive(Debug, Clone)]
pub enum EntryType {
    Card(PlayedCard),
}

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub ty: EntryType,
    pub active_target: Option<ActiveTarget>,
}

#[derive(Debug, Default)]
pub struct Stack {
    pub spells: BTreeMap<usize, StackEntry>,
    next_id: usize,
}

impl Stack {
    pub fn push_card(&mut self, card: PlayedCard, target: Option<ActiveTarget>) {
        let id = self.next_id;
        self.next_id += 1;
        self.spells.insert(
            id,
            StackEntry {
                ty: EntryType::Card(card),
                active_target: target,
            },
        );
    }

    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }

    pub fn resolve_1(&mut self, battlefield: &mut Battlefield) {
        let (_, next) = self
            .spells
            .last_key_value()
            .expect("Stack shouldn't be empty.");
        let next = next.clone();

        match &next.ty {
            EntryType::Card(card) => {
                for effect in card.card.effects.iter() {
                    match effect {
                        Effect::CounterSpell { target } => {
                            match next.active_target {
                                Some(active_target) => {
                                    match active_target {
                                        ActiveTarget::Stack { id } => {
                                            let Some(maybe_target) = &self.spells.get(&id) else {
                                                // Spell has left the stack already
                                                return;
                                            };

                                            match &maybe_target.ty {
                                                EntryType::Card(maybe_target) => {
                                                    if !maybe_target.card.can_be_countered(
                                                        battlefield,
                                                        &card.controller.borrow(),
                                                        &maybe_target.controller.borrow(),
                                                        target,
                                                    ) {
                                                        // Spell is no longer a valid target.
                                                        return;
                                                    }
                                                }
                                            }

                                            // If we reach here, we know the spell can be countered.
                                            self.spells.remove(&id);
                                        }
                                    }
                                }
                                None => {
                                    // Spell fizzles due to lack of target.
                                    return;
                                }
                            };
                        }
                        Effect::GainMana(_) => todo!(),
                        Effect::ModifyBasePT {
                            targets: _,
                            base_power: _,
                            base_toughness: _,
                        } => todo!(),
                        _ => {}
                    }
                }
            }
        }
    }
}
