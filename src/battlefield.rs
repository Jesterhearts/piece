use std::collections::{HashMap, HashSet};

use enumset::enum_set;
use indexmap::{IndexMap, IndexSet};

use crate::{
    abilities::{ETBAbility, StaticAbility},
    controller::Controller,
    cost::AdditionalCost,
    effects::{
        BattlefieldModifier, EffectDuration, ModifyBasePowerToughness, ModifyBattlefield,
        RemoveAllSubtypes,
    },
    in_play::{AllCards, AllModifiers, CardId, EffectsInPlay, ModifierId, ModifierInPlay},
    player::PlayerRef,
    stack::{ActiveTarget, Stack},
    targets::Restriction,
    types::{Subtype, Type},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    TapPermanent(CardId),
    PermanentToGraveyard(CardId),
    AddToStack(CardId, EffectsInPlay, Option<ActiveTarget>),
    CloneCreatureNonTargeting {
        source: CardId,
        target: Option<CardId>,
    },
    AddModifier {
        source: CardId,
        modifier: ModifierId,
    },
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ModifierSource {
    UntilEndOfTurn,
    Card(CardId),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Permanent {
    pub tapped: bool,
}

#[derive(Debug, Default)]
pub struct Battlefield {
    pub permanents: IndexMap<CardId, Permanent>,
    pub graveyards: HashMap<PlayerRef, IndexSet<CardId>>,
    pub exiles: HashMap<PlayerRef, IndexSet<CardId>>,

    pub sourced_modifiers: IndexMap<ModifierSource, HashSet<ModifierId>>,
    pub attaching_modifiers: IndexMap<CardId, HashSet<ModifierId>>,
    pub attached_cards: IndexMap<CardId, HashSet<CardId>>,
}

impl Battlefield {
    pub fn is_empty(&self) -> bool {
        self.permanents.is_empty() && self.no_modifiers()
    }

    pub fn no_modifiers(&self) -> bool {
        self.sourced_modifiers.is_empty()
            && self.attaching_modifiers.is_empty()
            && self.attached_cards.values().all(|v| v.is_empty())
    }

    #[must_use]
    pub fn add(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        source_card_id: CardId,
    ) -> Vec<ActionResult> {
        let mut result = vec![];
        self.permanents
            .insert(source_card_id, Permanent { tapped: false });

        if cards[source_card_id].face_down {
            let modifier_id = modifiers.add_modifier(ModifierInPlay {
                modifier: BattlefieldModifier {
                    modifier: ModifyBattlefield::ModifyBasePowerToughness(
                        ModifyBasePowerToughness {
                            targets: vec![],
                            power: 2,
                            toughness: 2,
                            restrictions: enum_set!(Restriction::SingleTarget),
                        },
                    ),
                    controller: Controller::Any,
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                },
                controller: cards[source_card_id].controller.clone(),
                modifying: vec![],
            });

            self.sourced_modifiers
                .entry(ModifierSource::Card(source_card_id))
                .or_default()
                .insert(modifier_id);

            let modifier_id = modifiers.add_modifier(ModifierInPlay {
                modifier: BattlefieldModifier {
                    modifier: ModifyBattlefield::RemoveAllSubtypes(RemoveAllSubtypes {
                        restrictions: enum_set!(Restriction::SingleTarget),
                    }),
                    controller: Controller::Any,
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                },
                controller: cards[source_card_id].controller.clone(),
                modifying: vec![],
            });

            self.sourced_modifiers
                .entry(ModifierSource::Card(source_card_id))
                .or_default()
                .insert(modifier_id);
        }

        let card = &cards[source_card_id];
        for etb in card.card.etb_abilities.iter() {
            match etb {
                ETBAbility::CopyOfAnyCreature => {
                    result.push(ActionResult::CloneCreatureNonTargeting {
                        source: source_card_id,
                        target: None,
                    });
                }
            }
        }

        for ability in card.card.static_abilities.iter() {
            match ability {
                StaticAbility::GreenCannotBeCountered { .. } => {}
                StaticAbility::Vigilance => {}
                StaticAbility::BattlefieldModifier(modifier) => {
                    let modifier_id = modifiers.add_modifier(ModifierInPlay {
                        modifier: modifier.clone(),
                        controller: card.controller.clone(),
                        modifying: Default::default(),
                    });
                    result.push(ActionResult::AddModifier {
                        source: source_card_id,
                        modifier: modifier_id,
                    })
                }
                StaticAbility::Enchant(_) => todo!(),
            }
        }

        for (source, sourced_modifiers) in self.sourced_modifiers.iter() {
            match source {
                ModifierSource::UntilEndOfTurn => {}
                ModifierSource::Card(id) => {
                    for modifier_id in sourced_modifiers.iter().copied() {
                        apply_modifier_to_targets(
                            modifiers,
                            modifier_id,
                            std::iter::once(source_card_id),
                            cards,
                            *id,
                        );
                    }
                }
            }
        }

        result
    }

    pub fn end_turn(&mut self, cards: &mut AllCards, modifers: &mut AllModifiers) {
        for effect in self
            .sourced_modifiers
            .get_mut(&ModifierSource::UntilEndOfTurn)
            .unwrap_or(&mut Default::default())
            .drain()
        {
            let modifier = modifers.remove(effect);
            for card_id in modifier.modifying {
                cards[card_id]
                    .card
                    .remove_modifier(effect, &modifier.modifier);
            }
        }
    }

    #[must_use]
    pub fn check_sba(&self, cards: &AllCards) -> Vec<ActionResult> {
        let mut result = vec![];
        for card_id in self.permanents.keys() {
            let card = &cards[*card_id].card;

            if (card.toughness.is_some() || !card.adjusted_base_toughness.is_empty())
                && card.toughness() <= 0
            {
                result.push(ActionResult::PermanentToGraveyard(*card_id));
            }
        }

        result
    }

    pub fn select_card(&self, index: usize) -> CardId {
        *self.permanents.get_index(index).unwrap().0
    }

    #[must_use]
    pub fn activate_ability(
        &self,
        card_id: CardId,
        cards: &AllCards,
        stack: &Stack,
        index: usize,
        target: Option<ActiveTarget>,
    ) -> Vec<ActionResult> {
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

            results.push(ActionResult::TapPermanent(card_id));
        }

        for cost in ability.cost.additional_cost.iter() {
            match cost {
                AdditionalCost::SacrificeThis => {
                    if !card.card.can_be_sacrificed(self) {
                        return vec![];
                    }

                    results.push(ActionResult::PermanentToGraveyard(card_id));
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

        results.push(ActionResult::AddToStack(
            card_id,
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

    pub fn apply_action_results(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        stack: &mut Stack,
        results: Vec<ActionResult>,
    ) {
        for result in results {
            match result {
                ActionResult::TapPermanent(card_id) => {
                    let permanent = self.permanents.get_mut(&card_id).unwrap();
                    assert!(!permanent.tapped);
                    permanent.tapped = true;
                }
                ActionResult::PermanentToGraveyard(card_id) => {
                    self.permanent_to_graveyard(cards, modifiers, stack, card_id);
                }
                ActionResult::AddToStack(source, effects, target) => {
                    stack.push_activated_ability(source, effects, target);
                }
                ActionResult::CloneCreatureNonTargeting { source, target } => {
                    if let Some(target) = target {
                        cards[source].card = cards[target].card.clone();
                    }
                }
                ActionResult::AddModifier { source, modifier } => {
                    self.apply_modifier(cards, modifiers, source, modifier);
                }
            }
        }
    }

    pub fn permanent_to_graveyard(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        stack: &mut Stack,
        card_id: CardId,
    ) {
        self.permanents.remove(&card_id).unwrap();
        cards[card_id].controller = cards[card_id].owner.clone();
        self.graveyards
            .entry(cards[card_id].owner.clone())
            .or_default()
            .insert(card_id);

        self.card_leaves_battlefield(card_id, modifiers, cards, stack);
    }

    fn card_leaves_battlefield(
        &mut self,
        removed_card_id: CardId,
        modifiers: &mut AllModifiers,
        cards: &mut AllCards,
        stack: &mut Stack,
    ) {
        if let Some(removed_modifiers) = self
            .sourced_modifiers
            .remove(&ModifierSource::Card(removed_card_id))
        {
            for modifier_id in removed_modifiers {
                let modifier = modifiers.remove(modifier_id);
                for card in modifier.modifying.iter().copied() {
                    cards[card]
                        .card
                        .remove_modifier(modifier_id, &modifier.modifier)
                }
            }
        }

        if let Some(removed_modifiers) = self.attaching_modifiers.remove(&removed_card_id) {
            for modifier_id in removed_modifiers {
                let modifier = modifiers.remove(modifier_id);
                for modified_card in modifier.modifying.iter().copied() {
                    self.attached_cards
                        .entry(modified_card)
                        .or_default()
                        .remove(&removed_card_id);

                    cards[modified_card]
                        .card
                        .remove_modifier(modifier_id, &modifier.modifier)
                }
            }
        }

        if let Some(attached_cards) = self.attached_cards.remove(&removed_card_id) {
            for card in attached_cards {
                if cards[card].card.subtypes_intersect(&[Subtype::Aura]) {
                    self.permanent_to_graveyard(cards, modifiers, stack, card);
                }
            }
        }
    }

    pub fn apply_modifier(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        source_card_id: CardId,
        modifier_id: ModifierId,
    ) {
        Self::apply_modifier_to_targets_internal(
            &mut self.sourced_modifiers,
            &mut self.attaching_modifiers,
            &mut self.attached_cards,
            cards,
            modifiers,
            source_card_id,
            modifier_id,
            self.permanents.keys().copied(),
        );
    }

    pub fn apply_modifier_to_targets(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        source_card_id: CardId,
        modifier_id: ModifierId,
        targets: Vec<CardId>,
    ) {
        Self::apply_modifier_to_targets_internal(
            &mut self.sourced_modifiers,
            &mut self.attaching_modifiers,
            &mut self.attached_cards,
            cards,
            modifiers,
            source_card_id,
            modifier_id,
            targets.into_iter(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_modifier_to_targets_internal(
        sourced_modifiers: &mut IndexMap<ModifierSource, HashSet<ModifierId>>,
        attaching_modifiers: &mut IndexMap<CardId, HashSet<ModifierId>>,
        attached_modifiers: &mut IndexMap<CardId, HashSet<CardId>>,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        source_card_id: CardId,
        modifier_id: ModifierId,
        targets: impl Iterator<Item = CardId> + Clone,
    ) {
        apply_modifier_to_targets(
            modifiers,
            modifier_id,
            targets.clone(),
            cards,
            source_card_id,
        );

        let modifier = &mut modifiers[modifier_id];

        match modifier.modifier.duration {
            EffectDuration::UntilEndOfTurn => {
                sourced_modifiers
                    .entry(ModifierSource::UntilEndOfTurn)
                    .or_default()
                    .insert(modifier_id);
            }
            EffectDuration::UntilSourceLeavesBattlefield => {
                sourced_modifiers
                    .entry(ModifierSource::Card(source_card_id))
                    .or_default()
                    .insert(modifier_id);
            }
            EffectDuration::UntilUnattached => {
                attaching_modifiers
                    .entry(source_card_id)
                    .or_default()
                    .insert(modifier_id);
                for target in targets {
                    attached_modifiers
                        .entry(target)
                        .or_default()
                        .insert(source_card_id);
                }
            }
        }
    }

    pub(crate) fn creatures(&self, cards: &AllCards) -> Vec<CardId> {
        self.permanents
            .keys()
            .copied()
            .filter(move |card_id| {
                let card = &cards[*card_id].card;
                card.types.contains(&Type::Creature)
            })
            .collect()
    }

    pub(crate) fn get(&self, id: CardId) -> Option<CardId> {
        if self.permanents.contains_key(&id) {
            Some(id)
        } else {
            None
        }
    }

    pub(crate) fn exile(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        stack: &mut Stack,
        target: CardId,
    ) {
        let removed = self.permanents.remove(&target);
        assert!(removed.is_some());

        let card = &mut cards[target];
        card.controller = card.owner.clone();

        self.exiles
            .entry(card.controller.clone())
            .or_default()
            .insert(target);

        self.card_leaves_battlefield(target, modifiers, cards, stack);
    }
}

fn apply_modifier_to_targets(
    modifiers: &mut AllModifiers,
    modifier_id: ModifierId,
    targets: impl Iterator<Item = CardId>,
    cards: &mut AllCards,
    source_card_id: CardId,
) {
    let modifier = &mut modifiers[modifier_id];

    'outer: for card_id in targets {
        let card = &mut cards[card_id];
        match modifier.modifier.controller {
            Controller::Any => {}
            Controller::You => {
                if modifier.controller != card.controller {
                    continue;
                }
            }
            Controller::Opponent => {
                if modifier.controller == card.controller {
                    continue;
                }
            }
        }

        for restriction in modifier.modifier.restrictions().iter() {
            match restriction {
                Restriction::NotSelf => {
                    if card_id == source_card_id {
                        continue 'outer;
                    }
                }
                Restriction::SingleTarget => {
                    if !modifier.modifying.is_empty() {
                        assert_eq!(modifier.modifying.len(), 1);
                        continue 'outer;
                    }
                }
                Restriction::CreaturesOnly => {
                    if !card.card.types_intersect(&[Type::Creature]) {
                        continue 'outer;
                    }
                }
                Restriction::ControllerControlsBlackOrGreen => todo!(),
            }
        }

        card.card.add_modifier(modifier_id, &modifier.modifier);
        modifier.modifying.push(card_id);
    }
}
