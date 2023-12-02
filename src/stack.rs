use indexmap::IndexMap;

use crate::{
    battlefield::Battlefield,
    card::CastingModifier,
    effects::Effect,
    in_play::{AllCards, CardId, CreaturesModifier, EffectsInPlay, ModifierInPlay},
    player::PlayerRef,
};

#[derive(Debug, PartialEq, Clone)]
pub enum StackResult {
    AddToBattlefield(CardId),
    ApplyToBattlefield(ModifierInPlay),
    ModifyCreatures(CreaturesModifier),
    SpellCountered { id: usize },
    RemoveSplitSecond,
    DrawCards { player: PlayerRef, count: usize },
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ActiveTarget {
    Stack { id: usize },
    Battlefield { id: CardId },
}

#[derive(Debug, Clone)]
pub enum EntryType {
    Card(CardId),
    ActivatedAbility(EffectsInPlay),
}

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub ty: EntryType,
    pub active_target: Option<ActiveTarget>,
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
            },
        );

        id
    }
    pub(crate) fn push_activatetd_ability(
        &mut self,
        effects: EffectsInPlay,
        target: Option<ActiveTarget>,
    ) {
        let id = self.next_id;
        self.next_id += 1;
        self.stack.insert(
            id,
            StackEntry {
                ty: EntryType::ActivatedAbility(effects),
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
    pub fn resolve_1(&mut self, cards: &AllCards, battlefield: &Battlefield) -> Vec<StackResult> {
        let mut result = vec![];
        let (_, next) = self.stack.pop().expect("Stack shouldn't be empty.");

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
                                                EntryType::ActivatedAbility(_) => {
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
                        Effect::GainMana { mana: _ } => todo!(),
                        Effect::BattlefieldModifier(_) => todo!(),
                        Effect::ControllerDrawCards(_) => todo!(),
                        Effect::Equip(_) => todo!(),
                        Effect::AddPowerToughness(_) => todo!(),
                    }
                }

                if resolving_card.card.is_permanent() {
                    result.push(StackResult::AddToBattlefield(card));
                }

                result
            }
            EntryType::ActivatedAbility(effects) => {
                for effect in effects.effects.into_iter() {
                    match effect {
                        Effect::CounterSpell { target: _ } => todo!(),
                        Effect::GainMana { mana: _ } => todo!(),
                        Effect::BattlefieldModifier(modifier) => {
                            result.push(StackResult::ApplyToBattlefield(ModifierInPlay {
                                modifier,
                                controller: effects.controller.clone(),
                                modified_cards: Default::default(),
                            }));
                        }
                        Effect::ControllerDrawCards(count) => {
                            result.push(StackResult::DrawCards {
                                player: effects.controller.clone(),
                                count,
                            });
                        }
                        Effect::Equip(modifier) => {
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
                                    let card = &cards[id];
                                    if !card.card.can_be_targeted(
                                        cards,
                                        effects.source,
                                        &card.controller.borrow(),
                                    ) {
                                        // Card is not a valid target, spell fizzles.
                                        return vec![];
                                    }

                                    result.push(StackResult::ModifyCreatures(CreaturesModifier {
                                        source: effects.source,
                                        effect: modifier,
                                        targets: vec![id],
                                        modified_cards: Default::default(),
                                    }));
                                }
                            }
                        }
                        Effect::AddPowerToughness(_) => todo!(),
                    }
                }
                result
            }
        }
    }
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

        stack.push_card(&all_cards, creature, None);
        let result = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(result, [StackResult::AddToBattlefield(creature)]);

        assert!(stack.is_empty());

        Ok(())
    }

    #[test]
    fn resolves_counterspells() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::empty());
        let mut all_cards = AllCards::default();
        let battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let counterspell_1 = all_cards.add(&cards, player.clone(), "Counterspell");
        let counterspell_2 = all_cards.add(&cards, player.clone(), "Counterspell");

        let countered = stack.push_card(&all_cards, counterspell_1, None);

        stack.push_card(&all_cards, counterspell_2, stack.target_nth(0));

        assert_eq!(stack.stack.len(), 2);

        let result = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(result, [StackResult::SpellCountered { id: countered }]);

        Ok(())
    }

    #[test]
    fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::empty());
        let mut all_cards = AllCards::default();
        let battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let creature = all_cards.add(&cards, player.clone(), "Allosaurus Shepherd");
        let counterspell = all_cards.add(&cards, player.clone(), "Counterspell");

        stack.push_card(&all_cards, creature, None);
        stack.push_card(&all_cards, counterspell, stack.target_nth(0));

        assert_eq!(stack.stack.len(), 2);

        let result = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(result, []);

        assert_eq!(stack.stack.len(), 1);

        Ok(())
    }

    #[test]
    fn does_not_resolve_counterspells_respecting_green_uncounterable() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player = Player::new_ref(Deck::empty());
        let mut all_cards = AllCards::default();
        let mut battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let creature_1 = all_cards.add(&cards, player.clone(), "Allosaurus Shepherd");
        let creature_2 = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
        let counterspell = all_cards.add(&cards, player.clone(), "Counterspell");

        stack.push_card(&all_cards, creature_1, None);
        let mut result = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(result, [StackResult::AddToBattlefield(creature_1)]);

        let Some(StackResult::AddToBattlefield(card)) = result.pop() else {
            unreachable!()
        };
        battlefield.add(card);

        stack.push_card(&all_cards, creature_2, None);
        stack.push_card(&all_cards, counterspell, stack.target_nth(0));

        assert_eq!(stack.stack.len(), 2);

        let result = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(result, []);

        assert_eq!(stack.stack.len(), 1);

        let result = stack.resolve_1(&all_cards, &battlefield);
        assert!(stack.is_empty());
        assert_eq!(result, [StackResult::AddToBattlefield(creature_2)]);

        Ok(())
    }

    #[test]
    fn resolves_counterspells_respecting_green_uncounterable_other_player() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let player1 = Player::new_ref(Deck::empty());
        let player2 = Player::new_ref(Deck::empty());

        let mut all_cards = AllCards::default();
        let mut battlefield = Battlefield::default();
        let mut stack = Stack::default();

        let creature_1 = all_cards.add(&cards, player1.clone(), "Allosaurus Shepherd");
        let creature_2 = all_cards.add(&cards, player2.clone(), "Alpine Grizzly");
        let counterspell = all_cards.add(&cards, player1.clone(), "Counterspell");

        stack.push_card(&all_cards, creature_1, None);
        let mut result = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(result, [StackResult::AddToBattlefield(creature_1)]);

        let Some(StackResult::AddToBattlefield(card)) = result.pop() else {
            unreachable!()
        };
        battlefield.add(card);

        let countered = stack.push_card(&all_cards, creature_2, None);
        stack.push_card(&all_cards, counterspell, stack.target_nth(0));

        assert_eq!(stack.stack.len(), 2);

        let result = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(result, [StackResult::SpellCountered { id: countered }]);

        Ok(())
    }
}
