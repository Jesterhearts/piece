use std::collections::BTreeMap;

use crate::{
    battlefield::Battlefield,
    card::{Effect, PlayedCard},
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

    pub fn target_nth(&self, nth: usize) -> Option<ActiveTarget> {
        self.spells
            .keys()
            .copied()
            .nth(nth)
            .map(|id| ActiveTarget::Stack { id })
    }

    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }

    pub fn resolve_1(&mut self, battlefield: &mut Battlefield) {
        let (_, next) = self.spells.pop_last().expect("Stack shouldn't be empty.");

        match next.ty {
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
                        Effect::GainMana { mana: _ } => todo!(),
                        Effect::ModifyBasePT {
                            targets: _,
                            base_power: _,
                            base_toughness: _,
                        } => todo!(),
                        _ => {}
                    }
                }

                if card.card.is_permanent() {
                    battlefield.add(card)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use crate::{
        battlefield::{Battlefield, Permanent},
        card::PlayedCard,
        deck::Deck,
        load_cards,
        player::Player,
        stack::Stack,
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::new(vec![]), 0);

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

        stack.resolve_1(&mut battlefield);

        assert!(stack.is_empty());
        assert_eq!(
            battlefield.permanents[0],
            Permanent {
                card,
                tapped: false
            }
        );

        Ok(())
    }

    #[test]
    fn resolves_counterspells() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::new(vec![]), 0);

        let mut battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let counterspell = cards
            .get("Counterspell")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card, None);

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card.clone(), stack.target_nth(0));

        assert_eq!(stack.spells.len(), 2);

        stack.resolve_1(&mut battlefield);

        assert!(stack.is_empty());

        Ok(())
    }

    #[test]
    fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::new(vec![]), 0);

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

        let counterspell = cards
            .get("Counterspell")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card, stack.target_nth(0));

        assert_eq!(stack.spells.len(), 2);

        stack.resolve_1(&mut battlefield);

        assert_eq!(stack.spells.len(), 1);

        Ok(())
    }

    #[test]
    fn does_not_resolve_counterspells_respecting_green_uncounterable() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::new(vec![]), 0);

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
        stack.resolve_1(&mut battlefield);

        let creature = cards
            .get("Alpine Grizzly")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: creature.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card, None);

        let counterspell = cards
            .get("Counterspell")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card, stack.target_nth(0));

        assert_eq!(stack.spells.len(), 2);

        stack.resolve_1(&mut battlefield);

        assert_eq!(stack.spells.len(), 1);

        stack.resolve_1(&mut battlefield);
        assert!(stack.is_empty());
        assert_eq!(battlefield.permanents.len(), 2);
        assert_eq!(battlefield.permanents.last().unwrap().card.card, *creature);

        Ok(())
    }

    #[test]
    fn resolves_counterspells_respecting_green_uncounterable_other_player() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::new(vec![]), 0);
        let player2 = Player::new_ref(Deck::new(vec![]), 1);

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
        stack.resolve_1(&mut battlefield);

        let creature = cards
            .get("Alpine Grizzly")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: creature.clone(),
            controller: player2.clone(),
            owner: player2.clone(),
        };
        stack.push_card(card, None);

        let counterspell = cards
            .get("Counterspell")
            .ok_or_else(|| anyhow!("Failed to find test card"))?;

        let card = PlayedCard {
            card: counterspell.clone(),
            controller: player.clone(),
            owner: player.clone(),
        };
        stack.push_card(card, stack.target_nth(0));

        assert_eq!(stack.spells.len(), 2);

        stack.resolve_1(&mut battlefield);
        assert!(stack.is_empty());
        assert_eq!(battlefield.permanents.len(), 1);

        Ok(())
    }
}
