use std::{
    collections::{HashMap, HashSet},
    vec,
};

use enumset::{enum_set, EnumSet};
use indexmap::{IndexMap, IndexSet};

use crate::{
    abilities::{ETBAbility, StaticAbility},
    card::Color,
    controller::Controller,
    cost::AdditionalCost,
    effects::{
        BattlefieldModifier, EffectDuration, Mill, ModifyBasePowerToughness, ModifyBattlefield,
        RemoveAllSubtypes, ReturnFromGraveyardToBattlefield, ReturnFromGraveyardToLibrary, Token,
        TriggeredEffect,
    },
    in_play::{
        AllCards, AllModifiers, CardId, EffectsInPlay, ModifierId, ModifierInPlay, ModifierType,
    },
    player::PlayerRef,
    stack::{ActiveTarget, Stack},
    targets::Restriction,
    triggers::{Location, PutIntoGraveyard, Trigger},
    types::Type,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedActionResult {
    TapPermanent(CardId),
    PermanentToGraveyard(CardId),
    AddToStack {
        card: CardId,
        effects: EffectsInPlay,
        valid_targets: Vec<ActiveTarget>,
    },
    CloneCreatureNonTargeting {
        source: CardId,
        valid_targets: Vec<CardId>,
    },
    AddModifier {
        modifier: ModifierId,
    },
    Mill {
        count: usize,
        valid_targets: HashSet<PlayerRef>,
    },
    ReturnFromGraveyardToLibrary {
        count: usize,
        controller: Controller,
        types: EnumSet<Type>,
        valid_targets: Vec<CardId>,
    },
    ReturnFromGraveyardToBattlefield {
        count: usize,
        types: EnumSet<Type>,
        valid_targets: Vec<CardId>,
    },
    CreateToken {
        source: CardId,
        token: Token,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    TapPermanent(CardId),
    PermanentToGraveyard(CardId),
    AddToStack {
        card: CardId,
        effects: EffectsInPlay,
        target: Option<ActiveTarget>,
    },
    CloneCreatureNonTargeting {
        source: CardId,
        target: Option<CardId>,
    },
    AddModifier {
        modifier: ModifierId,
    },
    Mill {
        count: usize,
        targets: HashSet<PlayerRef>,
    },
    ReturnFromGraveyardToLibrary {
        targets: Vec<CardId>,
    },
    ReturnFromGraveyardToBattlefield {
        targets: Vec<CardId>,
    },
    CreateToken {
        source: CardId,
        token: Token,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub global_modifiers: IndexMap<ModifierSource, HashSet<ModifierId>>,
    pub attaching_modifiers: IndexMap<ModifierSource, HashSet<ModifierId>>,
    pub attached_cards: IndexMap<CardId, HashSet<CardId>>,

    pub on_graveyard_triggers: IndexMap<CardId, (PutIntoGraveyard, Vec<TriggeredEffect>)>,
}

impl Battlefield {
    pub fn is_empty(&self) -> bool {
        self.permanents.is_empty() && self.no_modifiers()
    }

    pub fn no_modifiers(&self) -> bool {
        self.global_modifiers.is_empty()
            && self.attaching_modifiers.is_empty()
            && self.attached_cards.values().all(|v| v.is_empty())
    }

    #[must_use]
    pub fn add(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        source_card_id: CardId,
        targets: Vec<CardId>,
    ) -> Vec<UnresolvedActionResult> {
        let mut results = vec![];
        let mut this = scopeguard::guard_on_success(self, |this| {
            this.permanents
                .insert(source_card_id, Permanent { tapped: false });
        });

        if cards[source_card_id].face_down {
            let modifier_id = modifiers.add_modifier(ModifierInPlay {
                source: source_card_id,
                modifier: BattlefieldModifier {
                    modifier: ModifyBattlefield::ModifyBasePowerToughness(
                        ModifyBasePowerToughness {
                            targets: enum_set!(),
                            power: 2,
                            toughness: 2,
                        },
                    ),
                    controller: Controller::Any,
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                    restrictions: enum_set!(Restriction::SingleTarget),
                },
                controller: cards[source_card_id].controller.clone(),
                modifying: vec![],
                modifier_type: ModifierType::CardProperty,
            });
            this.apply_modifier_to_targets(cards, modifiers, modifier_id, &[source_card_id]);

            let modifier_id = modifiers.add_modifier(ModifierInPlay {
                source: source_card_id,
                modifier: BattlefieldModifier {
                    modifier: ModifyBattlefield::RemoveAllSubtypes(RemoveAllSubtypes {}),
                    controller: Controller::Any,
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                    restrictions: enum_set!(Restriction::SingleTarget),
                },
                controller: cards[source_card_id].controller.clone(),
                modifying: vec![source_card_id],
                modifier_type: ModifierType::CardProperty,
            });
            this.apply_modifier_to_targets(cards, modifiers, modifier_id, &[source_card_id]);

            let modifier_id = modifiers.add_modifier(ModifierInPlay {
                source: source_card_id,
                modifier: BattlefieldModifier {
                    modifier: ModifyBattlefield::RemoveAllAbilities,
                    controller: Controller::Any,
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                    restrictions: enum_set!(Restriction::SingleTarget),
                },
                controller: cards[source_card_id].controller.clone(),
                modifying: vec![source_card_id],
                modifier_type: ModifierType::CardProperty,
            });

            this.apply_modifier_to_targets(cards, modifiers, modifier_id, &[source_card_id]);
        }

        if let Some(enchant) = cards[source_card_id].card.enchant.clone() {
            for modifier in enchant.modifiers {
                let modifier_id = modifiers.add_modifier(ModifierInPlay {
                    source: source_card_id,
                    modifier: modifier.clone(),
                    controller: cards[source_card_id].controller.clone(),
                    modifying: vec![],
                    modifier_type: ModifierType::Aura,
                });

                this.apply_modifier_to_targets(cards, modifiers, modifier_id, &targets);
            }
        }

        for etb in cards[source_card_id].card.etb_abilities.clone() {
            match etb {
                ETBAbility::CopyOfAnyCreature => {
                    assert!(targets.is_empty());

                    results.push(UnresolvedActionResult::CloneCreatureNonTargeting {
                        source: source_card_id,
                        valid_targets: this.creatures(cards),
                    });
                }
                ETBAbility::Mill(Mill { count, target }) => {
                    let targets = match target {
                        Controller::Any => cards.all_players(),
                        Controller::You => {
                            HashSet::from([cards[source_card_id].controller.clone()])
                        }
                        Controller::Opponent => {
                            let mut all = cards.all_players();
                            all.remove(&cards[source_card_id].controller);
                            all
                        }
                    };

                    results.push(UnresolvedActionResult::Mill {
                        count,
                        valid_targets: targets,
                    })
                }
                ETBAbility::ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary {
                    count,
                    controller,
                    types,
                }) => {
                    let target_cards = compute_graveyard_targets(
                        controller,
                        cards,
                        cards[source_card_id].controller.clone(),
                        &this,
                        types,
                    );

                    results.push(UnresolvedActionResult::ReturnFromGraveyardToLibrary {
                        count,
                        controller,
                        types,
                        valid_targets: target_cards,
                    });
                }
                ETBAbility::ReturnFromGraveyardToBattlefield(
                    ReturnFromGraveyardToBattlefield { count, types },
                ) => {
                    let target_cards = compute_graveyard_targets(
                        Controller::You,
                        cards,
                        cards[source_card_id].controller.clone(),
                        &this,
                        types,
                    );

                    results.push(UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
                        count,
                        types,
                        valid_targets: target_cards,
                    });
                }
            }
        }

        for ability in cards[source_card_id].card.static_abilities() {
            match ability {
                StaticAbility::GreenCannotBeCountered { .. } => {}
                StaticAbility::Vigilance => {}
                StaticAbility::BattlefieldModifier(modifier) => {
                    let modifier_id = modifiers.add_modifier(ModifierInPlay {
                        source: source_card_id,
                        modifier,
                        controller: cards[source_card_id].controller.clone(),
                        modifying: Default::default(),
                        modifier_type: ModifierType::Global,
                    });
                    results.push(UnresolvedActionResult::AddModifier {
                        modifier: modifier_id,
                    })
                }
            }
        }

        for (source, global_modifiers) in this.global_modifiers.iter() {
            match source {
                ModifierSource::UntilEndOfTurn => {}
                ModifierSource::Card(_) => {
                    for modifier_id in global_modifiers.iter().copied() {
                        apply_modifier_to_targets(
                            modifiers,
                            modifier_id,
                            std::iter::once(source_card_id),
                            cards,
                        );
                    }
                }
            }
        }

        for triggered in cards[source_card_id].card.triggered_abilities.iter() {
            match &triggered.trigger {
                Trigger::PutIntoGraveyard(gy) => {
                    this.on_graveyard_triggers
                        .insert(source_card_id, (gy.clone(), triggered.effects.clone()));
                }
            }
        }

        results
    }

    pub fn controlled_colors(&self, cards: &AllCards, player: PlayerRef) -> EnumSet<Color> {
        let mut colors = enum_set!();
        for permanent in self.permanents.keys() {
            let card = &cards[*permanent];
            if card.controller == player {
                colors.extend(card.card.color());
            }
        }

        colors
    }

    pub fn end_turn(&mut self, cards: &mut AllCards, modifiers: &mut AllModifiers) {
        for effect in self
            .global_modifiers
            .get_mut(&ModifierSource::UntilEndOfTurn)
            .unwrap_or(&mut Default::default())
            .drain()
        {
            let modifier = modifiers.remove(effect);
            for card_id in modifier.modifying {
                cards[card_id]
                    .card
                    .remove_modifier(effect, &modifier.modifier);
            }
        }

        for modifier_id in self
            .attaching_modifiers
            .remove(&ModifierSource::UntilEndOfTurn)
            .into_iter()
            .flat_map(|modifiers| modifiers.into_iter())
        {
            let modifier = modifiers.remove(modifier_id);
            for card_id in modifier.modifying {
                self.attached_cards
                    .get_mut(&card_id)
                    .expect("Attaching modifiers should have a corresponding attached card")
                    .remove(&modifier.source);

                cards[card_id]
                    .card
                    .remove_modifier(modifier_id, &modifier.modifier);
            }
        }
    }

    #[must_use]
    pub fn check_sba(&self, cards: &AllCards) -> Vec<ActionResult> {
        let mut result = vec![];
        for card_id in self.permanents.keys() {
            let card = &cards[*card_id].card;

            if card.toughness().is_some() && card.toughness() <= Some(0) {
                result.push(ActionResult::PermanentToGraveyard(*card_id));
            }

            if card.enchant.is_some()
                && !self
                    .attaching_modifiers
                    .contains_key(&ModifierSource::Card(*card_id))
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
    ) -> Vec<UnresolvedActionResult> {
        if stack.split_second {
            return vec![];
        }

        let mut results = vec![];

        let card = &cards[card_id];
        let ability = &card.card.activated_abilities()[index];

        if ability.cost.tap {
            if self.permanents.get(&card_id).unwrap().tapped {
                return vec![];
            }

            results.push(UnresolvedActionResult::TapPermanent(card_id));
        }

        for cost in ability.cost.additional_cost.iter() {
            match cost {
                AdditionalCost::SacrificeThis => {
                    if !card.card.can_be_sacrificed(self) {
                        return vec![];
                    }

                    results.push(UnresolvedActionResult::PermanentToGraveyard(card_id));
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

        results.push(UnresolvedActionResult::AddToStack {
            card: card_id,
            effects: EffectsInPlay {
                effects: ability.effects.clone(),
                source: card_id,
                controller: card.controller.clone(),
            },
            valid_targets: cards[card_id].card.valid_targets(
                cards,
                self,
                stack,
                &cards[card_id].controller.borrow(),
            ),
        });

        results
    }

    pub fn static_abilities(&self, cards: &AllCards) -> Vec<(StaticAbility, PlayerRef)> {
        let mut result: Vec<(StaticAbility, PlayerRef)> = Default::default();

        for (id, _) in self.permanents.iter() {
            let card = &cards[*id];
            for ability in card.card.static_abilities().into_iter() {
                result.push((ability, card.controller.clone()));
            }
        }

        result
    }

    /// Attempts to automatically resolve any unresolved actions and _recomputes targets for pending actions_.
    pub fn maybe_resolve(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        stack: &mut Stack,
        resolution_controller: PlayerRef,
        results: Vec<UnresolvedActionResult>,
    ) -> Vec<UnresolvedActionResult> {
        let mut pending = vec![];

        for result in results {
            match result {
                UnresolvedActionResult::TapPermanent(cardid) => {
                    pending.extend(self.apply_action_result(
                        ActionResult::TapPermanent(cardid),
                        cards,
                        modifiers,
                        stack,
                    ));
                }
                UnresolvedActionResult::PermanentToGraveyard(cardid) => {
                    pending.extend(self.apply_action_result(
                        ActionResult::PermanentToGraveyard(cardid),
                        cards,
                        modifiers,
                        stack,
                    ));
                }
                UnresolvedActionResult::AddToStack {
                    card,
                    effects,
                    mut valid_targets,
                } => {
                    let wants_targets: usize = effects
                        .effects
                        .iter()
                        .map(|effect| effect.wants_targets())
                        .max()
                        .unwrap();
                    if wants_targets >= valid_targets.len() {
                        pending.extend(self.apply_action_result(
                            ActionResult::AddToStack {
                                card,
                                effects,
                                target: valid_targets.pop(),
                            },
                            cards,
                            modifiers,
                            stack,
                        ));
                    } else {
                        pending.push(UnresolvedActionResult::AddToStack {
                            card,
                            effects,
                            valid_targets: cards[card].card.valid_targets(
                                cards,
                                self,
                                stack,
                                &cards[card].controller.borrow(),
                            ),
                        });
                    }
                }
                UnresolvedActionResult::CloneCreatureNonTargeting {
                    source,
                    valid_targets,
                } => {
                    pending.push(UnresolvedActionResult::CloneCreatureNonTargeting {
                        source,
                        valid_targets,
                    });
                }
                UnresolvedActionResult::AddModifier { modifier } => {
                    pending.extend(self.apply_action_results(
                        cards,
                        modifiers,
                        stack,
                        vec![ActionResult::AddModifier { modifier }],
                    ));
                }
                UnresolvedActionResult::Mill {
                    count,
                    valid_targets,
                } => {
                    if valid_targets.len() == 1 {
                        pending.extend(self.apply_action_result(
                            ActionResult::Mill {
                                count,
                                targets: valid_targets,
                            },
                            cards,
                            modifiers,
                            stack,
                        ));
                    } else {
                        pending.push(UnresolvedActionResult::Mill {
                            count,
                            valid_targets,
                        });
                    }
                }
                UnresolvedActionResult::ReturnFromGraveyardToLibrary {
                    count,
                    controller,
                    types,
                    valid_targets,
                } => {
                    if valid_targets.len() <= count {
                        pending.extend(self.apply_action_result(
                            ActionResult::ReturnFromGraveyardToLibrary {
                                targets: valid_targets,
                            },
                            cards,
                            modifiers,
                            stack,
                        ));
                    } else {
                        pending.push(UnresolvedActionResult::ReturnFromGraveyardToLibrary {
                            count,
                            controller,
                            types,
                            valid_targets: compute_graveyard_targets(
                                controller,
                                cards,
                                resolution_controller.clone(),
                                self,
                                types,
                            ),
                        })
                    }
                }
                UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
                    count,
                    types,
                    valid_targets,
                } => {
                    if valid_targets.len() <= count {
                        pending.extend(self.apply_action_result(
                            ActionResult::ReturnFromGraveyardToBattlefield {
                                targets: valid_targets,
                            },
                            cards,
                            modifiers,
                            stack,
                        ));
                    } else {
                        pending.push(UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
                            count,
                            types,
                            valid_targets: compute_graveyard_targets(
                                Controller::You,
                                cards,
                                resolution_controller.clone(),
                                self,
                                types,
                            ),
                        })
                    }
                }
                UnresolvedActionResult::CreateToken { source, token } => {
                    pending.extend(self.apply_action_result(
                        ActionResult::CreateToken { source, token },
                        cards,
                        modifiers,
                        stack,
                    ));
                }
            }
        }

        pending
    }

    #[must_use]
    pub fn apply_action_results(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        stack: &mut Stack,
        results: Vec<ActionResult>,
    ) -> Vec<UnresolvedActionResult> {
        let mut pending = vec![];

        for result in results {
            pending.extend(self.apply_action_result(result, cards, modifiers, stack));
        }
        pending
    }

    #[must_use]
    fn apply_action_result(
        &mut self,
        result: ActionResult,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        stack: &mut Stack,
    ) -> Vec<UnresolvedActionResult> {
        match result {
            ActionResult::TapPermanent(card_id) => {
                let permanent = self.permanents.get_mut(&card_id).unwrap();
                assert!(!permanent.tapped);
                permanent.tapped = true;
            }
            ActionResult::PermanentToGraveyard(card_id) => {
                return self.permanent_to_graveyard(cards, modifiers, stack, card_id);
            }
            ActionResult::AddToStack {
                card,
                effects,
                target,
            } => {
                stack.push_activated_ability(card, effects, target);
            }
            ActionResult::CloneCreatureNonTargeting { source, target } => {
                if let Some(target) = target {
                    cards[source].card = cards[target].card.clone();
                }
            }
            ActionResult::AddModifier { modifier } => {
                self.apply_modifier(cards, modifiers, modifier);
            }
            ActionResult::Mill { count, targets } => {
                for target in targets {
                    for _ in 0..count {
                        let card_id = target.borrow_mut().deck.draw();
                        if let Some(card_id) = card_id {
                            self.graveyards
                                .entry(cards[card_id].owner.clone())
                                .or_default()
                                .insert(card_id);
                        }
                    }
                }
            }
            ActionResult::ReturnFromGraveyardToLibrary { targets } => {
                for target in targets {
                    let owner = cards[target].owner.clone();
                    self.graveyards
                        .get_mut(&owner)
                        .expect("Card should be in a graveyard")
                        .remove(&target);

                    owner.borrow_mut().deck.place_on_top(target);
                }
            }
            ActionResult::ReturnFromGraveyardToBattlefield { targets } => {
                let mut pending = vec![];
                for target in targets {
                    let owner = cards[target].owner.clone();
                    self.graveyards
                        .get_mut(&owner)
                        .expect("Card should be in a graveyard")
                        .remove(&target);

                    pending.extend(self.add(cards, modifiers, target, vec![]));
                }

                return pending;
            }
            ActionResult::CreateToken { source, token } => {
                let controller = cards[source].controller.clone();
                let id = cards.add_token(controller, token);
                self.permanents.insert(id, Permanent { tapped: false });
            }
        }

        vec![]
    }

    #[must_use]
    pub fn permanent_to_graveyard(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        stack: &mut Stack,
        card_id: CardId,
    ) -> Vec<UnresolvedActionResult> {
        self.permanents.remove(&card_id).unwrap();
        cards[card_id].controller = cards[card_id].owner.clone();
        self.graveyards
            .entry(cards[card_id].owner.clone())
            .or_default()
            .insert(card_id);

        let mut pending = vec![];

        for (trigger_source, (filter, effects)) in self.on_graveyard_triggers.iter() {
            if matches!(filter.location, Location::Anywhere | Location::Battlefield)
                && cards[card_id].card.types_intersect(filter.types)
            {
                for effect in effects.iter() {
                    match effect {
                        TriggeredEffect::CreateToken(token) => {
                            pending.push(UnresolvedActionResult::CreateToken {
                                source: *trigger_source,
                                token: token.clone(),
                            })
                        }
                    }
                }
            }
        }

        self.card_leaves_battlefield(card_id, modifiers, cards, stack);

        pending
    }

    fn card_leaves_battlefield(
        &mut self,
        removed_card_id: CardId,
        modifiers: &mut AllModifiers,
        cards: &mut AllCards,
        _stack: &mut Stack,
    ) {
        if let Some(removed_modifiers) = self
            .global_modifiers
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

        if let Some(removed_modifiers) = self
            .attaching_modifiers
            .remove(&ModifierSource::Card(removed_card_id))
        {
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
                if let Some(attached_modifiers) =
                    self.attaching_modifiers.remove(&ModifierSource::Card(card))
                {
                    for modifier in attached_modifiers {
                        if modifiers[modifier].modifying.len() <= 1 {
                            modifiers.remove(modifier);
                        }
                    }
                }
            }
        }
    }

    pub fn apply_modifier(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        modifier_id: ModifierId,
    ) {
        Self::apply_modifier_to_targets_internal(
            &mut self.global_modifiers,
            &mut self.attaching_modifiers,
            &mut self.attached_cards,
            cards,
            modifiers,
            modifier_id,
            self.permanents.keys().copied(),
        );
    }

    pub fn apply_modifier_to_targets(
        &mut self,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        modifier_id: ModifierId,
        targets: &[CardId],
    ) {
        Self::apply_modifier_to_targets_internal(
            &mut self.global_modifiers,
            &mut self.attaching_modifiers,
            &mut self.attached_cards,
            cards,
            modifiers,
            modifier_id,
            targets.iter().copied(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_modifier_to_targets_internal(
        global_modifiers: &mut IndexMap<ModifierSource, HashSet<ModifierId>>,
        attaching_modifiers: &mut IndexMap<ModifierSource, HashSet<ModifierId>>,
        attached_modifiers: &mut IndexMap<CardId, HashSet<CardId>>,
        cards: &mut AllCards,
        modifiers: &mut AllModifiers,
        modifier_id: ModifierId,
        targets: impl Iterator<Item = CardId> + Clone,
    ) {
        apply_modifier_to_targets(modifiers, modifier_id, targets.clone(), cards);

        let modifier = &mut modifiers[modifier_id];

        match modifier.modifier_type {
            ModifierType::Global => match modifier.modifier.duration {
                EffectDuration::UntilEndOfTurn => {
                    global_modifiers
                        .entry(ModifierSource::UntilEndOfTurn)
                        .or_default()
                        .insert(modifier_id);
                }
                EffectDuration::UntilSourceLeavesBattlefield => {
                    global_modifiers
                        .entry(ModifierSource::Card(modifier.source))
                        .or_default()
                        .insert(modifier_id);
                }
            },
            ModifierType::Equipment
            | ModifierType::Aura
            | ModifierType::CardProperty
            | ModifierType::Temporary => match modifier.modifier.duration {
                EffectDuration::UntilEndOfTurn => {
                    attaching_modifiers
                        .entry(ModifierSource::UntilEndOfTurn)
                        .or_default()
                        .insert(modifier_id);
                    for target in targets {
                        attached_modifiers
                            .entry(target)
                            .or_default()
                            .insert(modifier.source);
                    }
                }
                EffectDuration::UntilSourceLeavesBattlefield => {
                    attaching_modifiers
                        .entry(ModifierSource::Card(modifier.source))
                        .or_default()
                        .insert(modifier_id);
                    for target in targets {
                        attached_modifiers
                            .entry(target)
                            .or_default()
                            .insert(modifier.source);
                    }
                }
            },
        }
    }

    pub(crate) fn creatures(&self, cards: &AllCards) -> Vec<CardId> {
        self.permanents
            .keys()
            .copied()
            .filter(move |card_id| {
                let card = &cards[*card_id].card;
                card.types().contains(Type::Creature)
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

fn compute_graveyard_targets(
    controller: Controller,
    cards: &mut AllCards,
    card_controller: PlayerRef,
    this: &Battlefield,
    types: EnumSet<Type>,
) -> Vec<CardId> {
    let targets = match controller {
        Controller::Any => cards.all_players(),
        Controller::You => HashSet::from([card_controller]),
        Controller::Opponent => {
            let mut all = cards.all_players();
            all.remove(&card_controller);
            all
        }
    };

    let mut target_cards = vec![];

    for target in targets {
        let graveyard = this.graveyards.get(&target);
        for card_id in graveyard
            .iter()
            .flat_map(|graveyard| graveyard.iter())
            .copied()
        {
            let card = &cards[card_id];
            if card.card.types_intersect(types) {
                target_cards.push(card_id);
            }
        }
    }
    target_cards
}

fn apply_modifier_to_targets(
    modifiers: &mut AllModifiers,
    modifier_id: ModifierId,
    targets: impl Iterator<Item = CardId>,
    cards: &mut AllCards,
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

        for restriction in modifier.modifier.restrictions.iter() {
            match restriction {
                Restriction::NotSelf => {
                    if card_id == modifier.source {
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
                    if !card.card.types_intersect(enum_set![Type::Creature]) {
                        continue 'outer;
                    }
                }
            }
        }

        card.card.add_modifier(modifier_id, &modifier.modifier);
        modifier.modifying.push(card_id);
    }
}
