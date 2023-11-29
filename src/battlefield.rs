use std::{
    collections::{HashMap, HashSet},
    vec,
};

use crate::{
    card::{PlayedCard, PlayedEffect, StaticAbility},
    mana::AdditionalCost,
    player::PlayerRef,
    stack::{ActiveTarget, Stack},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivatedAbilityResult {
    TapPermanent,
    PermanentToGraveyard,
    AddToStack(PlayedEffect, Option<ActiveTarget>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Permanent {
    pub card: PlayedCard,
    pub tapped: bool,
}

impl Permanent {
    pub fn activate_ability(
        &self,
        battlefield: &Battlefield,
        stack: &Stack,
        index: usize,
    ) -> Vec<ActivatedAbilityResult> {
        if stack.split_second {
            return vec![];
        }

        let mut results = vec![];

        let ability = self.card.card.activated_abilities[index].clone();

        if ability.cost.tap {
            if self.tapped {
                return vec![];
            }

            results.push(ActivatedAbilityResult::TapPermanent);
        }

        for cost in ability.cost.additional_costs {
            match cost {
                AdditionalCost::SacrificeThis => {
                    if !self.card.card.can_be_sacrificed(battlefield) {
                        return vec![];
                    }

                    results.push(ActivatedAbilityResult::PermanentToGraveyard);
                }
            }
        }

        if !self
            .card
            .controller
            .borrow_mut()
            .spend_mana(&ability.cost.mana)
        {
            return vec![];
        }

        results
    }
}

#[derive(Debug, Default)]
pub struct Battlefield {
    pub permanents: Vec<Permanent>,
    pub graveyards: Vec<Permanent>,
}

impl Battlefield {
    pub fn add(&mut self, played: PlayedCard) {
        self.permanents.push(Permanent {
            card: played,
            tapped: false,
        });
    }

    pub fn static_abilities(&self) -> HashMap<StaticAbility, HashSet<PlayerRef>> {
        let mut result: HashMap<StaticAbility, HashSet<PlayerRef>> = Default::default();

        for permanent in self.permanents.iter() {
            for ability in permanent.card.card.static_abilities.iter().cloned() {
                result
                    .entry(ability)
                    .or_default()
                    .insert(permanent.card.controller.clone());
            }
        }

        result
    }

    pub fn select_card(&self, index: usize) -> Permanent {
        self.permanents[index].clone()
    }

    pub fn apply_activated_ability(
        &mut self,
        stack: &mut Stack,
        index: usize,
        results: Vec<ActivatedAbilityResult>,
    ) {
        for result in results {
            match result {
                ActivatedAbilityResult::TapPermanent => {
                    let permanent = &mut self.permanents[index];
                    assert!(!permanent.tapped);
                    permanent.tapped = true;
                }
                ActivatedAbilityResult::PermanentToGraveyard => {
                    let mut permanent = self.permanents.remove(index);
                    permanent.card.controller = permanent.card.owner.clone();
                    self.graveyards.push(permanent);
                }
                ActivatedAbilityResult::AddToStack(effect, target) => {
                    stack.push_effect(effect, target);
                }
            }
        }
    }

    pub fn apply_results(
        &mut self,
        stack: &mut Stack,
        results: Vec<ActivatedAbilityResult>,
        index: usize,
    ) {
        for result in results {
            match result {
                ActivatedAbilityResult::TapPermanent => {
                    self.permanents[index].tapped = true;
                }
                ActivatedAbilityResult::PermanentToGraveyard => {}
                ActivatedAbilityResult::AddToStack(_, _) => todo!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        battlefield::{ActivatedAbilityResult, Battlefield},
        card::PlayedCard,
        deck::Deck,
        load_cards,
        player::Player,
        stack::Stack,
    };

    #[test]
    fn sacrifice_effects_work() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let stack = Stack::default();
        let mut battlefield = Battlefield::default();
        let player = Player::new_ref(Deck::empty(), 0);
        player.borrow_mut().infinite_mana();

        let card = cards.get("Abzan Banner").expect("Failed to find test card");

        battlefield.add(PlayedCard {
            card: card.clone(),
            controller: player.clone(),
            owner: player,
        });

        let card = battlefield.select_card(0);
        let result = card.activate_ability(&battlefield, &stack, 1);
        assert_eq!(
            result,
            [
                ActivatedAbilityResult::TapPermanent,
                ActivatedAbilityResult::PermanentToGraveyard
            ]
        );

        Ok(())
    }
}
