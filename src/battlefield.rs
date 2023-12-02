use std::{
    collections::{HashMap, HashSet},
    vec,
};

use indexmap::{IndexMap, IndexSet};

use crate::{
    abilities::StaticAbility,
    cost::AdditionalCost,
    effects::{
        AddPowerToughness, EffectDuration, ModifyBasePowerToughness, ModifyBattlefield,
        ModifyCreature, ModifyCreatureTypes,
    },
    in_play::{AllCards, CardId, CreaturesModifier, EffectsInPlay, ModifierInPlay},
    player::PlayerRef,
    stack::{ActiveTarget, Stack},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivatedAbilityResult {
    TapPermanent,
    PermanentToGraveyard,
    AddToStack(EffectsInPlay, Option<ActiveTarget>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Permanent {
    pub tapped: bool,
}

#[derive(Debug, Default)]
pub struct Battlefield {
    pub permanents: IndexMap<CardId, Permanent>,
    pub graveyards: HashMap<PlayerRef, IndexSet<CardId>>,

    pub until_end_of_turn: Vec<ModifierInPlay>,
    pub creature_modifiers: IndexMap<CardId, CreaturesModifier>,
}

impl Battlefield {
    pub fn add(&mut self, card: CardId) {
        self.permanents.insert(card, Permanent { tapped: false });
    }

    pub fn end_turn(&mut self, cards: &mut AllCards) {
        for effect in self.until_end_of_turn.drain(..).rev() {
            match effect.modifier.modifier {
                ModifyBattlefield::ModifyBasePowerToughness(_) => {
                    for (cardid, card) in effect.modified_cards.into_iter() {
                        cards[cardid].card = card;
                    }
                }
                ModifyBattlefield::ModifyCreatureTypes(_) => {
                    for (cardid, card) in effect.modified_cards.into_iter() {
                        cards[cardid].card = card;
                    }
                }
                ModifyBattlefield::AddPowerToughness(AddPowerToughness { power, toughness }) => {
                    for cardid in self.permanents.keys().copied() {
                        *cards[cardid]
                            .card
                            .power_modifier
                            .as_mut()
                            .expect("Modified creatures should have a power modifier") -= power;
                        *cards[cardid]
                            .card
                            .toughness_modifier
                            .as_mut()
                            .expect("Modified creatures should have a toughness modifier") -=
                            toughness;
                    }
                }
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

        results.push(ActivatedAbilityResult::AddToStack(
            EffectsInPlay {
                effects: ability.effects.clone(),
                source: card_id,
                controller: card.controller.clone(),
            },
            target,
        ));

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
                    self.permanent_to_graveyard(cards, stack, card_id);
                }
                ActivatedAbilityResult::AddToStack(effect, target) => {
                    stack.push_activatetd_ability(effect, target);
                }
            }
        }
    }

    pub fn permanent_to_graveyard(
        &mut self,
        cards: &mut AllCards,
        _stack: &mut Stack,
        card_id: CardId,
    ) {
        self.permanents.remove(&card_id).unwrap();
        cards[card_id].controller = cards[card_id].owner.clone();
        self.graveyards
            .entry(cards[card_id].owner.clone())
            .or_default()
            .insert(card_id);

        if let Some(modifier) = self.creature_modifiers.remove(&card_id) {
            match modifier.effect {
                ModifyCreature::ModifyBasePowerToughness(_) => {
                    for (cardid, card) in modifier.modified_cards.into_iter() {
                        cards[cardid].card = card;
                    }
                }
                ModifyCreature::ModifyCreatureTypes(_) => {
                    for (cardid, card) in modifier.modified_cards.into_iter() {
                        cards[cardid].card = card;
                    }
                }
                ModifyCreature::AddPowerToughness(AddPowerToughness { power, toughness }) => {
                    for cardid in modifier.targets {
                        *cards[cardid]
                            .card
                            .power_modifier
                            .as_mut()
                            .expect("Modified creatures should have a power modifier") -= power;
                        *cards[cardid]
                            .card
                            .toughness_modifier
                            .as_mut()
                            .expect("Modified creatures should have a toughness modifier") -=
                            toughness;
                    }
                }
            }
        }
    }

    pub fn apply_modifier(&mut self, cards: &mut AllCards, mut modifier: ModifierInPlay) {
        match &modifier.modifier.modifier {
            ModifyBattlefield::ModifyBasePowerToughness(ModifyBasePowerToughness {
                targets,
                power: base_power,
                toughness: base_tough,
            }) => {
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
            }
            ModifyBattlefield::ModifyCreatureTypes(ModifyCreatureTypes { targets, types }) => {
                for cardid in self.permanents.keys() {
                    let card = &mut cards[*cardid];

                    if card.card.subtypes_match(targets) {
                        modifier.modified_cards.insert(*cardid, card.card.clone());
                        card.card.subtypes.extend(types.iter().copied());
                    }
                }
            }
            ModifyBattlefield::AddPowerToughness(AddPowerToughness { power, toughness }) => {
                for cardid in self.permanents.keys() {
                    *cards[*cardid].card.power_modifier.get_or_insert(0) += power;
                    *cards[*cardid].card.toughness_modifier.get_or_insert(0) += toughness;
                }
            }
        }

        match modifier.modifier.duration {
            EffectDuration::UntilEndOfTurn => self.until_end_of_turn.push(modifier),
        }
    }

    pub fn modify_creatures(&mut self, all_cards: &mut AllCards, mut modifier: CreaturesModifier) {
        for cardid in modifier.targets.iter() {
            match &modifier.effect {
                ModifyCreature::ModifyBasePowerToughness(ModifyBasePowerToughness {
                    targets,
                    power,
                    toughness,
                }) => {
                    let card = &mut all_cards[*cardid];

                    if card.card.subtypes_match(targets) {
                        modifier.modified_cards.insert(*cardid, card.card.clone());

                        *card.card.power.get_or_insert(0) = *power;
                        *card.card.toughness.get_or_insert(0) = *toughness;
                    }
                }
                ModifyCreature::ModifyCreatureTypes(ModifyCreatureTypes { targets, types }) => {
                    let card = &mut all_cards[*cardid].card;
                    if card.subtypes_match(targets) {
                        modifier.modified_cards.insert(*cardid, card.clone());
                        card.subtypes.extend(types.iter().copied());
                    }
                }
                ModifyCreature::AddPowerToughness(AddPowerToughness { power, toughness }) => {
                    *all_cards[*cardid].card.power_modifier.get_or_insert(0) += power;
                    *all_cards[*cardid].card.toughness_modifier.get_or_insert(0) += toughness;
                }
            }
        }

        self.creature_modifiers.insert(modifier.source, modifier);
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        battlefield::{ActivatedAbilityResult, Battlefield},
        deck::Deck,
        effects::{
            AddPowerToughness, BattlefieldModifier, Effect, EffectDuration,
            ModifyBasePowerToughness, ModifyBattlefield, ModifyCreature, ModifyCreatureTypes,
        },
        in_play::{AllCards, CreaturesModifier, EffectsInPlay, ModifierInPlay},
        load_cards,
        player::Player,
        stack::{ActiveTarget, Stack, StackResult},
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
                    EffectsInPlay {
                        effects: vec![Effect::ControllerDrawCards(1)],
                        source: card,
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
                EffectsInPlay {
                    effects: vec![
                        Effect::BattlefieldModifier(BattlefieldModifier {
                            modifier: ModifyBattlefield::ModifyBasePowerToughness(
                                ModifyBasePowerToughness {
                                    targets: vec![Subtype::Elf],
                                    power: 5,
                                    toughness: 5,
                                }
                            ),
                            duration: EffectDuration::UntilEndOfTurn,
                        }),
                        Effect::BattlefieldModifier(BattlefieldModifier {
                            modifier: ModifyBattlefield::ModifyCreatureTypes(ModifyCreatureTypes {
                                targets: vec![Subtype::Elf],
                                types: vec![Subtype::Dinosaur],
                            }),

                            duration: EffectDuration::UntilEndOfTurn,
                        })
                    ],
                    source: card,
                    controller: player.clone(),
                },
                None
            )]
        );

        battlefield.apply_activated_ability(&mut all_cards, &mut stack, card, results);

        let results = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(
            results,
            [
                StackResult::ApplyToBattlefield(ModifierInPlay {
                    modifier: BattlefieldModifier {
                        modifier: ModifyBattlefield::ModifyBasePowerToughness(
                            ModifyBasePowerToughness {
                                targets: vec![Subtype::Elf],
                                power: 5,
                                toughness: 5,
                            }
                        ),
                        duration: EffectDuration::UntilEndOfTurn
                    },
                    controller: player.clone(),
                    modified_cards: Default::default(),
                }),
                StackResult::ApplyToBattlefield(ModifierInPlay {
                    modifier: BattlefieldModifier {
                        modifier: ModifyBattlefield::ModifyCreatureTypes(ModifyCreatureTypes {
                            targets: vec![Subtype::Elf],
                            types: vec![Subtype::Dinosaur],
                        }),
                        duration: EffectDuration::UntilEndOfTurn
                    },
                    controller: player.clone(),
                    modified_cards: Default::default(),
                })
            ]
        );

        let Some(StackResult::ApplyToBattlefield(effect)) = results.into_iter().next() else {
            unreachable!()
        };

        battlefield.apply_modifier(&mut all_cards, effect);
        let card = battlefield.select_card(0);
        let card = &all_cards[card];
        assert_eq!(card.card.power(), 5);
        assert_eq!(card.card.toughness(), 5);

        battlefield.end_turn(&mut all_cards);

        let card = battlefield.select_card(0);
        let card = &all_cards[card];
        assert_eq!(card.card.power(), 1);
        assert_eq!(card.card.toughness(), 1);

        Ok(())
    }

    #[test]
    fn equipment_works() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let mut all_cards = AllCards::default();
        let mut stack = Stack::default();
        let mut battlefield = Battlefield::default();
        let player = Player::new_ref(Deck::empty());
        player.borrow_mut().infinite_mana();

        let equipment = all_cards.add(&cards, player.clone(), "+2 Mace");
        battlefield.add(equipment);

        let creature = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
        battlefield.add(creature);

        let equipment = battlefield.select_card(0);
        let results = battlefield.activate_ability(
            equipment,
            &all_cards,
            &stack,
            0,
            Some(ActiveTarget::Battlefield { id: creature }),
        );

        assert_eq!(
            results,
            [ActivatedAbilityResult::AddToStack(
                EffectsInPlay {
                    effects: vec![Effect::Equip(ModifyCreature::AddPowerToughness(
                        AddPowerToughness {
                            power: 2,
                            toughness: 2
                        }
                    )),],
                    source: equipment,
                    controller: player.clone(),
                },
                Some(ActiveTarget::Battlefield { id: creature })
            )]
        );

        battlefield.apply_activated_ability(&mut all_cards, &mut stack, equipment, results);

        let results = stack.resolve_1(&all_cards, &battlefield);
        assert_eq!(
            results,
            [StackResult::ModifyCreatures(CreaturesModifier {
                source: equipment,
                effect: ModifyCreature::AddPowerToughness(AddPowerToughness {
                    power: 2,
                    toughness: 2
                }),
                targets: vec![creature],
                modified_cards: Default::default()
            })]
        );

        let Some(StackResult::ModifyCreatures(modifier)) = results.into_iter().next() else {
            unreachable!()
        };

        battlefield.modify_creatures(&mut all_cards, modifier);

        let card = &all_cards[creature];
        assert_eq!(card.card.power(), 6);
        assert_eq!(card.card.toughness(), 4);

        battlefield.permanent_to_graveyard(&mut all_cards, &mut stack, equipment);

        let card = &all_cards[creature];
        assert_eq!(card.card.power(), 4);
        assert_eq!(card.card.toughness(), 2);

        Ok(())
    }
}
