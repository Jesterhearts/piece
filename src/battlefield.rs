use std::{collections::HashSet, vec};

use indexmap::{IndexMap, IndexSet};

use crate::{
    card::StaticAbility,
    in_play::{AllCards, CardId, EffectInPlay},
    mana::AdditionalCost,
    player::PlayerRef,
    stack::{ActiveTarget, Stack},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivatedAbilityResult {
    TapPermanent,
    PermanentToGraveyard,
    AddToStack(EffectInPlay, Option<ActiveTarget>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Permanent {
    pub tapped: bool,
}

#[derive(Debug, Default)]
pub struct Battlefield {
    pub permanents: IndexMap<CardId, Permanent>,
    pub graveyards: IndexSet<CardId>,
}

impl Battlefield {
    pub fn add(&mut self, card: CardId) {
        self.permanents.insert(card, Permanent { tapped: false });
    }

    pub fn select_card(&self, index: usize) -> CardId {
        *self.permanents.get_index(index).unwrap().0
    }

    pub fn activate_ability(
        &self,
        card_id: CardId,
        cards: &AllCards,
        stack: &Stack,
        index: usize,
        target: Option<ActiveTarget>,
    ) -> Vec<ActivatedAbilityResult> {
        if stack.split_second {
            return vec![];
        }

        let mut results = vec![];

        let card = &cards[card_id];
        let ability = &card.card.activated_abilities[index];

        if ability.cost.tap {
            if self.permanents.get(&card_id).unwrap().tapped {
                return vec![];
            }

            results.push(ActivatedAbilityResult::TapPermanent);
        }

        for cost in ability.cost.additional_costs.iter() {
            match cost {
                AdditionalCost::SacrificeThis => {
                    if !card.card.can_be_sacrificed(self) {
                        return vec![];
                    }

                    results.push(ActivatedAbilityResult::PermanentToGraveyard);
                }
            }
        }

        if !card.controller.borrow_mut().spend_mana(&ability.cost.mana) {
            return vec![];
        }

        for effect in ability.effects.iter() {
            results.push(ActivatedAbilityResult::AddToStack(
                EffectInPlay {
                    effect: effect.clone(),
                    controller: card.controller.clone(),
                },
                target,
            ));
        }

        results
    }

    pub fn static_abilities(
        &self,
        cards: &AllCards,
    ) -> IndexMap<StaticAbility, HashSet<PlayerRef>> {
        let mut result: IndexMap<StaticAbility, HashSet<PlayerRef>> = Default::default();

        for (id, _) in self.permanents.iter() {
            let card = &cards[*id];
            for ability in card.card.static_abilities.iter().cloned() {
                result
                    .entry(ability)
                    .or_default()
                    .insert(card.controller.clone());
            }
        }

        result
    }
    pub fn apply_activated_ability(
        &mut self,
        cards: &mut AllCards,
        stack: &mut Stack,
        card_id: CardId,
        results: Vec<ActivatedAbilityResult>,
    ) {
        for result in results {
            match result {
                ActivatedAbilityResult::TapPermanent => {
                    let permanent = self.permanents.get_mut(&card_id).unwrap();
                    assert!(!permanent.tapped);
                    permanent.tapped = true;
                }
                ActivatedAbilityResult::PermanentToGraveyard => {
                    self.permanents.remove(&card_id).unwrap();
                    cards[card_id].controller = cards[card_id].owner.clone();
                    self.graveyards.insert(card_id);
                }
                ActivatedAbilityResult::AddToStack(effect, target) => {
                    stack.push_effect(effect, target);
                }
            }
        }
    }

    pub fn apply_results(
        &mut self,
        cards: &mut AllCards,
        stack: &mut Stack,
        results: Vec<ActivatedAbilityResult>,
        id: CardId,
    ) {
        for result in results {
            match result {
                ActivatedAbilityResult::TapPermanent => {
                    self.permanents.get_mut(&id).unwrap().tapped = true;
                }
                ActivatedAbilityResult::PermanentToGraveyard => {
                    self.permanents.remove(&id).unwrap();
                    cards[id].controller = cards[id].owner.clone();
                    self.graveyards.insert(id);
                }
                ActivatedAbilityResult::AddToStack(effect, target) => {
                    stack.push_effect(effect, target);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        battlefield::{ActivatedAbilityResult, Battlefield},
        card::Effect,
        deck::Deck,
        in_play::{AllCards, EffectInPlay},
        load_cards,
        player::Player,
        stack::Stack,
    };

    #[test]
    fn sacrifice_effects_work() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let mut all_cards = AllCards::default();
        let stack = Stack::default();
        let mut battlefield = Battlefield::default();
        let player = Player::new_ref(Deck::empty());
        player.borrow_mut().infinite_mana();

        let card = all_cards.add(&cards, player.clone(), "Abzan Banner");
        battlefield.add(card);

        let card = battlefield.select_card(0);
        let result = battlefield.activate_ability(card, &all_cards, &stack, 1, None);
        assert_eq!(
            result,
            [
                ActivatedAbilityResult::TapPermanent,
                ActivatedAbilityResult::PermanentToGraveyard,
                ActivatedAbilityResult::AddToStack(
                    EffectInPlay {
                        effect: Effect::ControllerDrawCards(1),
                        controller: player
                    },
                    None
                )
            ]
        );

        Ok(())
    }
}
