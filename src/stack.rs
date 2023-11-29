use std::collections::BTreeMap;

use crate::{
    battlefield::Battlefield,
    card::{CastingModifier, Effect, PlayedCard, PlayedEffect},
    player::PlayerRef,
};

#[derive(Debug, PartialEq, Clone)]
pub enum StackResult {
    AddToBattlefield(PlayedCard),
    PlayerLoses(PlayerRef),
    SpellCountered { id: usize },
    RemoveSplitSecond,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ActiveTarget {
    Stack { id: usize },
}

#[derive(Debug, Clone)]
pub enum EntryType {
    Card(PlayedCard),
    Effect(PlayedEffect),
}

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub ty: EntryType,
    pub active_target: Option<ActiveTarget>,
}

#[derive(Debug, Default)]
pub struct Stack {
    pub stack: BTreeMap<usize, StackEntry>,
    next_id: usize,
    pub split_second: bool,
}

impl Stack {
    pub fn push_card(&mut self, card: PlayedCard, target: Option<ActiveTarget>) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        if card
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
            },
        );

        id
    }
    pub(crate) fn push_effect(&mut self, effect: PlayedEffect, target: Option<ActiveTarget>) {
        let id = self.next_id;
        self.next_id += 1;
        self.stack.insert(
            id,
            StackEntry {
                ty: EntryType::Effect(effect),
                active_target: target,
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
    pub fn resolve_1(&mut self, battlefield: &Battlefield) -> Vec<StackResult> {
        let mut result = vec![];
        let (_, next) = self.stack.pop_last().expect("Stack shouldn't be empty.");

        match next.ty {
            EntryType::Card(card) => {
                if card
                    .card
                    .casting_modifiers
                    .contains(&CastingModifier::SplitSecond)
                {
                    result.push(StackResult::RemoveSplitSecond);
                }

                for effect in card.card.effects.iter() {
                    match effect {
                        Effect::CounterSpell { target } => {
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
                                                    if !maybe_target.card.can_be_countered(
                                                        battlefield,
                                                        &card.controller.borrow(),
                                                        &maybe_target.controller.borrow(),
                                                        target,
                                                    ) {
                                                        // Spell is no longer a valid target.
                                                        return vec![];
                                                    }
                                                }
                                                EntryType::Effect(_) => todo!(),
                                            }

                                            // If we reach here, we know the spell can be countered.
                                            result.push(StackResult::SpellCountered { id });
                                        }
                                    }
                                }
                                None => {
                                    // Spell fizzles due to lack of target.
                                    return vec![];
                                }
                            };
                        }
                        Effect::GainMana { mana: _ } => todo!(),
                        Effect::ModifyBasePT {
                            targets: _,
                            base_power: _,
                            base_toughness: _,
                        } => todo!(),
                        Effect::ControllerDrawCards(_) => todo!(),
                    }
                }

                if card.card.is_permanent() {
                    result.push(StackResult::AddToBattlefield(card));
                }

                result
            }
            EntryType::Effect(effect) => match effect.effect {
                Effect::CounterSpell { target: _ } => todo!(),
                Effect::GainMana { mana: _ } => todo!(),
                Effect::ModifyBasePT {
                    targets: _,
                    base_power: _,
                    base_toughness: _,
                } => todo!(),
                Effect::ControllerDrawCards(count) => {
                    let controller = effect.controller;
                    if !controller.borrow_mut().draw(count) {
                        result.push(StackResult::PlayerLoses(controller.clone()));
                    }

                    result
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use crate::{
        battlefield::Battlefield,
        card::PlayedCard,
        deck::Deck,
        load_cards,
        player::Player,
        stack::{Stack, StackResult},
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::empty(), 0);

        let battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let creature = cards
            .get("Allosaurus Shepherd")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: creature.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card.clone(), None);

        let result = stack.resolve_1(&battlefield);
        assert_eq!(result, [StackResult::AddToBattlefield(card)]);

        assert!(stack.is_empty());

        Ok(())
    }

    #[test]
    fn resolves_counterspells() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::empty(), 0);

        let battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let counterspell = cards
            .get("Counterspell")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        let countered = stack.push_card(card, None);

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card.clone(), stack.target_nth(0));

        assert_eq!(stack.stack.len(), 2);

        let result = stack.resolve_1(&battlefield);
        assert_eq!(result, [StackResult::SpellCountered { id: countered }]);

        Ok(())
    }

    #[test]
    fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::empty(), 0);

        let battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let creature = cards
            .get("Allosaurus Shepherd")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: creature.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card.clone(), None);

        let counterspell = cards
            .get("Counterspell")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card, stack.target_nth(0));

        assert_eq!(stack.stack.len(), 2);

        let result = stack.resolve_1(&battlefield);
        assert_eq!(result, []);

        assert_eq!(stack.stack.len(), 1);

        Ok(())
    }

    #[test]
    fn does_not_resolve_counterspells_respecting_green_uncounterable() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::empty(), 0);

        let mut battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let creature = cards
            .get("Allosaurus Shepherd")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: creature.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card.clone(), None);
        let mut result = stack.resolve_1(&battlefield);
        assert_eq!(result, [StackResult::AddToBattlefield(card)]);

        let Some(StackResult::AddToBattlefield(card)) = result.pop() else {
            unreachable!()
        };
        battlefield.add(card);

        let creature = cards
            .get("Alpine Grizzly")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let creature = PlayedCard {
            card: creature.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(creature.clone(), None);

        let counterspell = cards
            .get("Counterspell")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card.clone(), stack.target_nth(0));

        assert_eq!(stack.stack.len(), 2);

        let result = stack.resolve_1(&battlefield);
        assert_eq!(result, []);

        assert_eq!(stack.stack.len(), 1);

        let result = stack.resolve_1(&battlefield);
        assert!(stack.is_empty());
        assert_eq!(result, [StackResult::AddToBattlefield(creature)]);

        Ok(())
    }

    #[test]
    fn resolves_counterspells_respecting_green_uncounterable_other_player() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::empty(), 0);
        let player2 = Player::new_ref(Deck::empty(), 1);

        let mut battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let creature = cards
            .get("Allosaurus Shepherd")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: creature.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card.clone(), None);
        let mut result = stack.resolve_1(&battlefield);
        assert_eq!(result, [StackResult::AddToBattlefield(card)]);

        let Some(StackResult::AddToBattlefield(card)) = result.pop() else {
            unreachable!()
        };
        battlefield.add(card);

        let creature = cards
            .get("Alpine Grizzly")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: creature.clone(),
            controller: player2.clone(),
            owner: player2.clone(),
        };
        let countered = stack.push_card(card, None);

        let counterspell = cards
            .get("Counterspell")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card, stack.target_nth(0));

        assert_eq!(stack.stack.len(), 2);

        let result = stack.resolve_1(&battlefield);
        assert_eq!(result, [StackResult::SpellCountered { id: countered }]);

        Ok(())
    }
}
