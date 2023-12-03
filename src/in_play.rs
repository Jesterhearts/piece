use std::collections::{HashMap, VecDeque};

use indexmap::IndexMap;

use crate::{
    abilities::StaticAbility,
    card::Card,
    deck::{Deck, DeckDefinition},
    effects::{ActivatedAbilityEffect, BattlefieldModifier},
    player::PlayerRef,
    Cards,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CardInPlay {
    pub card: Card,
    pub original_card: Card,
    pub controller: PlayerRef,
    pub owner: PlayerRef,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EffectsInPlay {
    pub effects: Vec<ActivatedAbilityEffect>,
    pub source: CardId,
    pub controller: PlayerRef,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AbilityInPlay {
    pub ability: StaticAbility,
    pub controller: PlayerRef,
    pub modified_cards: IndexMap<CardId, Card>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ModifierInPlay {
    pub modifier: BattlefieldModifier,
    pub controller: PlayerRef,
    pub modifying: Vec<CardId>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct CardId(usize);

#[derive(Default)]
pub struct AllCards {
    pub cards: HashMap<CardId, CardInPlay>,
    next_id: usize,
}

impl std::ops::Index<CardId> for AllCards {
    type Output = CardInPlay;

    fn index(&self, index: CardId) -> &Self::Output {
        self.cards.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<CardId> for AllCards {
    fn index_mut(&mut self, index: CardId) -> &mut Self::Output {
        self.cards.get_mut(&index).unwrap()
    }
}

impl AllCards {
    pub fn add_deck(&mut self, cards: &Cards, definition: &DeckDefinition) -> Deck {
        let mut deck = VecDeque::default();
        for (card, count) in definition.cards.iter() {
            for _ in 0..*count {
                let id = self.add(cards, definition.owner.clone(), card);
                deck.push_back(id);
            }
        }

        Deck::new(deck)
    }

    #[must_use]
    pub fn add(&mut self, cards: &Cards, owner: PlayerRef, name: &str) -> CardId {
        let id = self.next_id();
        self.cards.insert(
            id,
            CardInPlay {
                card: cards.get(name).expect("Valid card name").clone(),
                original_card: cards.get(name).expect("Valid card name").clone(),
                controller: owner.clone(),
                owner,
            },
        );
        id
    }

    fn next_id(&mut self) -> CardId {
        let id = self.next_id;
        self.next_id += 1;
        CardId(id)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct AbilityId(usize);

#[derive(Default)]
pub struct AllAbilities {
    pub abilities: HashMap<AbilityId, AbilityInPlay>,
    next_id: usize,
}

impl std::ops::Index<AbilityId> for AllAbilities {
    type Output = AbilityInPlay;

    fn index(&self, index: AbilityId) -> &Self::Output {
        self.abilities.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<AbilityId> for AllAbilities {
    fn index_mut(&mut self, index: AbilityId) -> &mut Self::Output {
        self.abilities.get_mut(&index).unwrap()
    }
}

impl AllAbilities {
    pub fn add_ability(&mut self, ability: AbilityInPlay) -> AbilityId {
        let id = self.next_id();
        self.abilities.insert(id, ability);
        id
    }
    fn next_id(&mut self) -> AbilityId {
        let id = self.next_id;
        self.next_id += 1;
        AbilityId(id)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ModifierId(usize);

#[derive(Default)]
pub struct AllModifiers {
    pub modifiers: HashMap<ModifierId, ModifierInPlay>,
    next_id: usize,
}

impl std::ops::Index<ModifierId> for AllModifiers {
    type Output = ModifierInPlay;

    fn index(&self, index: ModifierId) -> &Self::Output {
        self.modifiers.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<ModifierId> for AllModifiers {
    fn index_mut(&mut self, index: ModifierId) -> &mut Self::Output {
        self.modifiers.get_mut(&index).unwrap()
    }
}

impl AllModifiers {
    pub fn add_modifier(&mut self, modifier: ModifierInPlay) -> ModifierId {
        let id = self.next_id();
        self.modifiers.insert(id, modifier);
        id
    }

    pub fn remove(&mut self, id: ModifierId) -> ModifierInPlay {
        self.modifiers.remove(&id).unwrap()
    }

    fn next_id(&mut self) -> ModifierId {
        let id = self.next_id;
        self.next_id += 1;
        ModifierId(id)
    }
}
