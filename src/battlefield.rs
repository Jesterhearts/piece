use std::{collections::HashSet, vec};

use indexmap::{IndexMap, IndexSet};

use crate::{
    abilities::StaticAbility,
    cost::AdditionalCost,
    effects::{EffectDuration, ModifyBattlefield},
    in_play::{AllCards, CardId, EffectInPlay, ModifierInPlay},
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

    pub until_end_of_turn: Vec<ModifierInPlay>,
}

impl Battlefield {
    pub fn add(&mut self, card: CardId) {
        self.permanents.insert(card, Permanent { tapped: false });
    }

    pub fn end_turn(&mut self, cards: &mut AllCards) {
        for effect in self.until_end_of_turn.drain(..).rev() {
            for (cardid, card) in effect.modified_cards.into_iter() {
                cards[cardid].card = card;
            }
        }
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

        for cost in ability.cost.additional_cost.iter() {
            match cost {
                AdditionalCost::SacrificeThis => {
                    if !card.card.can_be_sacrificed(self) {
                        return vec![];
                    }

                    results.push(ActivatedAbilityResult::PermanentToGraveyard);
                }
            }
        }

        if !card
            .controller
            .borrow_mut()
            .spend_mana(&ability.cost.mana_cost)
        {
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

    fn apply(&mut self, cards: &mut AllCards, mut modifier: ModifierInPlay) {
        match &modifier.modifier {
            ModifyBattlefield::ModifyBasePowerToughness {
                targets,
                power: base_power,
                toughness: base_tough,
                duration,
            } => {
                for cardid in self.permanents.keys() {
                    let card = &mut cards[*cardid];

                    if card.card.subtypes_match(targets) {
                        modifier.modified_cards.insert(*cardid, card.card.clone());

                        if let Some(power) = &mut card.card.power {
                            *power = *base_power;
                        }
                        if let Some(tough) = &mut card.card.toughness {
                            *tough = *base_tough;
                        }
                    }
                }
                match duration {
                    EffectDuration::UntilEndOfTurn => self.until_end_of_turn.push(modifier),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        battlefield::{ActivatedAbilityResult, Battlefield},
        deck::Deck,
        effects::{Effect, EffectDuration, ModifyBattlefield},
        in_play::{AllCards, EffectInPlay, ModifierInPlay},
        load_cards,
        player::Player,
        stack::{Stack, StackResult},
        types::Subtype,
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

    #[test]
    fn modify_base_p_t_works() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let mut all_cards = AllCards::default();
        let mut stack = Stack::default();
        let mut battlefield = Battlefield::default();
        let player = Player::new_ref(Deck::empty());
        player.borrow_mut().infinite_mana();

        let card = all_cards.add(&cards, player.clone(), "Allosaurus Shepherd");
        battlefield.add(card);

        let card = battlefield.select_card(0);
        let results = battlefield.activate_ability(card, &all_cards, &stack, 0, None);

        assert_eq!(
            results,
            [ActivatedAbilityResult::AddToStack(
                EffectInPlay {
                    effect: Effect::ModifyBattlefield(
                        ModifyBattlefield::ModifyBasePowerToughness {
                            targets: vec![Subtype::Elf],
                            power: 5,
                            toughness: 5,
                            duration: EffectDuration::UntilEndOfTurn
                        }
                    ),
                    controller: player.clone()
                },
                None
            )]
        );

        battlefield.apply_activated_ability(&mut all_cards, &mut stack, card, results);

        let results = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(
            results,
            [StackResult::ApplyToBattlefield(ModifierInPlay {
                modifier: ModifyBattlefield::ModifyBasePowerToughness {
                    targets: vec![Subtype::Elf],
                    power: 5,
                    toughness: 5,
                    duration: EffectDuration::UntilEndOfTurn
                },
                controller: player,
                modified_cards: Default::default(),
            })]
        );

        let Some(StackResult::ApplyToBattlefield(effect)) = results.into_iter().next() else {
            unreachable!()
        };

        battlefield.apply(&mut all_cards, effect);
        let card = battlefield.select_card(0);
        let card = &all_cards[card];
        assert_eq!(card.card.power, Some(5));
        assert_eq!(card.card.toughness, Some(5));

        battlefield.end_turn(&mut all_cards);

        let card = battlefield.select_card(0);
        let card = &all_cards[card];
        assert_eq!(card.card.power, Some(1));
        assert_eq!(card.card.toughness, Some(1));

        Ok(())
    }
}
