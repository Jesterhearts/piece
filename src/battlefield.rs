use std::collections::{HashSet, VecDeque};

use bevy_ecs::{entity::Entity, query::With};
use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    abilities::{Ability, GainMana, GainManaAbility, StaticAbility},
    card::Color,
    controller::ControllerRestriction,
    cost::AdditionalCost,
    effects::{
        effect_duration::UntilEndOfTurn, replacing, BattlefieldModifier, Counter, Destination,
        Effect, EffectDuration, Mill, Token, TutorLibrary,
    },
    in_play::{
        all_cards, cards, AbilityId, Active, AuraId, CardId, CounterId, Database, InGraveyard,
        InLibrary, ModifierId, OnBattlefield, ReplacementEffectId, TriggerId,
    },
    mana::Mana,
    player::{AllPlayers, Controller, Owner},
    stack::{ActiveTarget, Entry, Stack, StackEntry},
    targets::Restriction,
    triggers::{self, trigger_source},
    types::Type,
};

#[derive(Debug, PartialEq, Eq)]

pub enum ResolutionResult {
    Complete,
    TryAgain,
    PendingChoice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingResult {
    pub apply_immediately: Vec<ActionResult>,
    pub then_resolve: VecDeque<UnresolvedAction>,
    pub recompute: bool,
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PendingResults {
    pub results: VecDeque<PendingResult>,
}

impl<const T: usize> From<[ActionResult; T]> for PendingResults {
    fn from(value: [ActionResult; T]) -> Self {
        Self {
            results: VecDeque::from([PendingResult {
                apply_immediately: value.to_vec(),
                then_resolve: Default::default(),
                recompute: false,
            }]),
        }
    }
}

impl PendingResults {
    pub fn push_resolved(&mut self, action: ActionResult) {
        if let Some(last) = self.results.back_mut() {
            if !last.then_resolve.is_empty() {
                self.results.push_back(PendingResult {
                    apply_immediately: vec![action],
                    then_resolve: Default::default(),
                    recompute: false,
                });
            } else {
                last.apply_immediately.push(action);
            }
        } else {
            self.results.push_back(PendingResult {
                apply_immediately: vec![action],
                then_resolve: Default::default(),
                recompute: false,
            });
        }
    }

    pub fn push_unresolved(&mut self, action: UnresolvedAction) {
        if let Some(last) = self.results.back_mut() {
            if !last.apply_immediately.is_empty() || !last.then_resolve.is_empty() {
                last.recompute = true;
            }
            last.then_resolve.push_back(action);
        } else {
            self.results.push_back(PendingResult {
                apply_immediately: Default::default(),
                then_resolve: VecDeque::from([action]),
                recompute: false,
            });
        }
    }

    pub fn is_optional(&self, db: &mut Database) -> bool {
        if let Some(to_resolve) = self.results.front() {
            if let Some(to_resolve) = to_resolve.then_resolve.front() {
                to_resolve.optional
                    || (to_resolve.source.is_some()
                        && to_resolve
                            .result
                            .wants_targets(db, to_resolve.source.unwrap())
                            >= to_resolve.choices.len())
            } else {
                true
            }
        } else {
            true
        }
    }

    pub fn options(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        if let Some(to_resolve) = self.results.front() {
            if let Some(to_resolve) = to_resolve.then_resolve.front() {
                to_resolve.choices(db, all_players)
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    #[must_use]
    pub fn resolve(
        &mut self,
        db: &mut Database,
        all_players: &mut AllPlayers,
        choice: Option<usize>,
    ) -> ResolutionResult {
        if self.results.is_empty() {
            return ResolutionResult::Complete;
        }

        let first = self.results.front_mut().unwrap();
        let results = Battlefield::apply_action_results(db, all_players, &first.apply_immediately);
        first.apply_immediately.clear();

        let extra = if let Some(to_resolve) = first.then_resolve.front_mut() {
            let actions = to_resolve.resolve(db, choice);
            if !actions.is_empty() {
                first.then_resolve.pop_front();
                Battlefield::apply_action_results(db, all_players, &actions)
            } else {
                self.extend(results);
                return ResolutionResult::PendingChoice;
            }
        } else {
            PendingResults::default()
        };

        if first.recompute {
            for to_resolve in first.then_resolve.iter_mut() {
                to_resolve.compute_targets(db);
            }
        }

        if first.then_resolve.is_empty() {
            self.results.pop_front();
        }

        self.extend(results);
        self.extend(extra);

        if self.results.is_empty() {
            ResolutionResult::Complete
        } else {
            ResolutionResult::TryAgain
        }
    }

    pub(crate) fn extend(&mut self, results: PendingResults) {
        self.results.extend(results.results);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub(crate) fn only_immediate_results(&self) -> bool {
        self.is_empty()
            || (self.results.len() == 1 && self.results.front().unwrap().then_resolve.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedActionResult {
    Effect(Effect),
    Attach(AuraId, CardId),
    Ability(AbilityId),
    AddCardToStack,
    AddTriggerToStack(TriggerId),
    OrganizeStack(Vec<StackEntry>),
}

impl UnresolvedActionResult {
    fn wants_targets(&self, db: &mut Database, source: CardId) -> usize {
        match self {
            UnresolvedActionResult::Effect(effect) => effect.wants_targets(),
            UnresolvedActionResult::Attach(_, _) => 1,
            UnresolvedActionResult::Ability(ability) => {
                let effects = ability.effects(db);
                let controller = ability.controller(db);
                effects
                    .into_iter()
                    .map(|effect| effect.into_effect(db, controller))
                    .map(|effect| effect.wants_targets())
                    .sum()
            }
            UnresolvedActionResult::AddCardToStack => source.wants_targets(db),
            UnresolvedActionResult::AddTriggerToStack(trigger) => trigger.wants_targets(db, source),
            UnresolvedActionResult::OrganizeStack(_) => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedAction {
    pub source: Option<CardId>,
    pub result: UnresolvedActionResult,
    pub valid_targets: Vec<ActiveTarget>,
    pub choices: IndexMap<Option<usize>, usize>,
    pub optional: bool,
}

impl UnresolvedAction {
    pub fn compute_targets(&mut self, db: &mut Database) {
        if let Some(source) = self.source {
            let controller = source.controller(db);
            match &self.result {
                UnresolvedActionResult::Effect(effect) => {
                    let creatures = Battlefield::creatures(db);

                    let mut valid_targets = vec![];
                    source.targets_for_effect(
                        db,
                        controller,
                        effect,
                        &creatures,
                        &mut valid_targets,
                    );

                    self.valid_targets = valid_targets;
                }
                UnresolvedActionResult::Ability(ability) => {
                    let ability = ability.ability(db);
                    let creatures = Battlefield::creatures(db);

                    let mut valid_targets = vec![];
                    for effect in ability.into_effects() {
                        let effect = effect.into_effect(db, controller);

                        source.targets_for_effect(
                            db,
                            controller,
                            &effect,
                            &creatures,
                            &mut valid_targets,
                        );
                    }

                    self.valid_targets = valid_targets;
                }
                UnresolvedActionResult::Attach(_, card) => {
                    self.valid_targets = card.valid_targets(db);
                }
                UnresolvedActionResult::AddCardToStack => {
                    let creatures = Battlefield::creatures(db);

                    let mut valid_targets = vec![];
                    for effect in source.effects(db) {
                        let effect = effect.into_effect(db, controller);

                        source.targets_for_effect(
                            db,
                            controller,
                            &effect,
                            &creatures,
                            &mut valid_targets,
                        );
                    }

                    self.valid_targets = valid_targets;
                }
                UnresolvedActionResult::AddTriggerToStack(trigger) => {
                    let creatures = Battlefield::creatures(db);

                    let mut valid_targets = vec![];
                    for effect in trigger.effects(db) {
                        let effect = effect.into_effect(db, controller);

                        source.targets_for_effect(
                            db,
                            controller,
                            &effect,
                            &creatures,
                            &mut valid_targets,
                        );
                    }

                    self.valid_targets = valid_targets;
                }
                UnresolvedActionResult::OrganizeStack(_) => {}
            }
        }
    }

    pub fn choices(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        match &self.result {
            UnresolvedActionResult::Effect(effect) => effect
                .choices(db, all_players, &self.valid_targets)
                .into_iter()
                .enumerate()
                .filter(|(idx, _)| !self.choices.contains_key(&Some(*idx)))
                .collect_vec(),
            UnresolvedActionResult::Ability(ability) => {
                let controller = self.source.unwrap().controller(db);
                ability
                    .ability(db)
                    .choices(db, all_players, controller, &self.valid_targets)
                    .into_iter()
                    .enumerate()
                    .filter(|(idx, _)| !self.choices.contains_key(&Some(*idx)))
                    .collect_vec()
            }
            UnresolvedActionResult::Attach(_, _)
            | UnresolvedActionResult::AddCardToStack
            | UnresolvedActionResult::AddTriggerToStack(_) => self
                .valid_targets
                .iter()
                .map(|target| target.display(db, all_players))
                .enumerate()
                .filter(|(idx, _)| !self.choices.contains_key(&Some(*idx)))
                .collect_vec(),
            UnresolvedActionResult::OrganizeStack(entries) => entries
                .iter()
                .map(|e| e.display(db))
                .enumerate()
                .filter(|(idx, _)| !self.choices.contains_key(&Some(*idx)))
                .collect_vec(),
        }
    }

    pub fn resolve(&mut self, db: &mut Database, choice: Option<usize>) -> Vec<ActionResult> {
        *self.choices.entry(choice).or_default() += 1;

        match &self.result {
            UnresolvedActionResult::Effect(effect) => match effect {
                Effect::CopyOfAnyCreatureNonTargeting => {
                    vec![ActionResult::CloneCreatureNonTargeting {
                        source: self.source.unwrap(),
                        target: choice.map(|choice| self.valid_targets[choice]),
                    }]
                }
                Effect::CounterSpell { .. } => {
                    todo!()
                }
                Effect::CreateToken(token) => vec![ActionResult::CreateToken {
                    source: self.source.unwrap().controller(db),
                    token: token.clone(),
                }],
                Effect::DealDamage(_) => todo!(),
                Effect::Equip(modifiers) => {
                    if choice.is_none() && !self.optional && self.valid_targets.len() > 1 {
                        return vec![];
                    }
                    let mut results = vec![];

                    // Hack.
                    self.source.unwrap().deactivate_modifiers(db);
                    for modifier in modifiers {
                        let id = ModifierId::upload_temporary_modifier(
                            db,
                            self.source.unwrap(),
                            &BattlefieldModifier {
                                modifier: modifier.clone(),
                                controller: ControllerRestriction::You,
                                duration: EffectDuration::UntilSourceLeavesBattlefield,
                                restrictions: vec![],
                            },
                        );
                        results.push(ActionResult::ApplyModifierToTarget {
                            modifier: id,
                            target: choice
                                .map_or(self.valid_targets[0], |choice| self.valid_targets[choice]),
                        })
                    }

                    results
                }
                Effect::ExileTargetCreature => {
                    if choice.is_none() && !self.optional && self.valid_targets.len() > 1 {
                        return vec![];
                    }

                    vec![ActionResult::ExileTarget(choice.map_or_else(
                        || self.valid_targets[0],
                        |choice| self.valid_targets[choice],
                    ))]
                }
                Effect::ExileTargetCreatureManifestTopOfLibrary => todo!(),
                Effect::GainCounter(_) => todo!(),
                Effect::Mill(Mill { count, .. }) => {
                    let mut targets = self
                        .choices
                        .iter()
                        .filter_map(|(target, count)| {
                            target.map(|target| std::iter::repeat(target).take(*count))
                        })
                        .flatten()
                        .map(|target| self.valid_targets[target])
                        .collect_vec();

                    if targets.is_empty() && self.valid_targets.len() == 1 {
                        targets = self.valid_targets.clone();
                    }

                    let wants_targets = effect.wants_targets();
                    if targets.len() >= wants_targets || targets.len() >= self.valid_targets.len() {
                        vec![ActionResult::Mill {
                            count: *count,
                            targets,
                        }]
                    } else {
                        vec![]
                    }
                }
                Effect::ModifyCreature(_) => todo!(),
                Effect::ReturnFromGraveyardToBattlefield(_) => {
                    let mut targets = self
                        .choices
                        .iter()
                        .filter_map(|(target, count)| {
                            target.map(|target| std::iter::repeat(target).take(*count))
                        })
                        .flatten()
                        .map(|target| self.valid_targets[target])
                        .collect_vec();

                    if targets.is_empty() && self.valid_targets.len() == 1 {
                        targets = self.valid_targets.clone();
                    }

                    let wants_targets = effect.wants_targets();
                    if targets.len() >= wants_targets || targets.len() >= self.valid_targets.len() {
                        vec![ActionResult::ReturnFromGraveyardToBattlefield { targets }]
                    } else {
                        vec![]
                    }
                }
                Effect::ReturnFromGraveyardToLibrary(_) => {
                    let mut targets = self
                        .choices
                        .iter()
                        .filter_map(|(target, count)| {
                            target.map(|target| std::iter::repeat(target).take(*count))
                        })
                        .flatten()
                        .map(|target| self.valid_targets[target])
                        .collect_vec();

                    if targets.is_empty() && self.valid_targets.len() == 1 {
                        targets = self.valid_targets.clone();
                    }

                    let wants_targets = effect.wants_targets();
                    if targets.len() >= wants_targets || targets.len() >= self.valid_targets.len() {
                        vec![ActionResult::ReturnFromGraveyardToLibrary { targets }]
                    } else {
                        vec![]
                    }
                }
                Effect::TutorLibrary(TutorLibrary {
                    destination,
                    reveal,
                    ..
                }) => {
                    let mut targets = self
                        .choices
                        .iter()
                        .filter_map(|(target, count)| {
                            target.map(|target| std::iter::repeat(target).take(*count))
                        })
                        .flatten()
                        .map(|target| self.valid_targets[target])
                        .collect_vec();

                    if targets.is_empty() && self.valid_targets.len() == 1 {
                        targets = self.valid_targets.clone();
                    }

                    let wants_targets = effect.wants_targets();
                    if targets.len() >= wants_targets || targets.len() >= self.valid_targets.len() {
                        let mut results = vec![];
                        let targets = targets
                            .into_iter()
                            .map(|target| {
                                let ActiveTarget::Library { id } = target else {
                                    unreachable!()
                                };
                                id
                            })
                            .collect_vec();
                        if *reveal {
                            for card in targets.iter() {
                                results.push(ActionResult::RevealCard(*card))
                            }
                        }

                        match destination {
                            Destination::Hand => {
                                for card in targets.iter() {
                                    results.push(ActionResult::MoveToHandFromLibrary(*card));
                                }
                            }
                            Destination::TopOfLibrary => todo!(),
                            Destination::Battlefield { enters_tapped } => {
                                for card in targets.iter() {
                                    results.push(ActionResult::AddToBattlefieldFromLibrary {
                                        card: *card,
                                        enters_tapped: *enters_tapped,
                                    });
                                }
                            }
                        }

                        results.push(ActionResult::Shuffle(self.source.unwrap().owner(db)));

                        results
                    } else {
                        vec![]
                    }
                }
                Effect::BattlefieldModifier(_)
                | Effect::ControllerDrawCards(_)
                | Effect::ControllerLosesLife(_) => {
                    unreachable!()
                }
            },
            UnresolvedActionResult::Ability(ability) => {
                if let Ability::Mana(GainManaAbility { gain, .. }) = ability.ability(db) {
                    match gain {
                        GainMana::Specific { gains } => {
                            vec![ActionResult::GainMana {
                                gain: gains,
                                target: self.source.unwrap().controller(db),
                            }]
                        }
                        GainMana::Choice { choices } => {
                            assert_eq!(self.choices.len(), 1);
                            if choice.is_none() {
                                self.choices.pop();
                                return vec![];
                            }

                            vec![ActionResult::GainMana {
                                gain: choices[choice.unwrap()].clone(),
                                target: self.source.unwrap().controller(db),
                            }]
                        }
                    }
                } else {
                    let targets = self
                        .choices
                        .iter()
                        .filter_map(|(target, count)| {
                            target.map(|target| std::iter::repeat(target).take(*count))
                        })
                        .flatten()
                        .map(|target| self.valid_targets[target])
                        .collect_vec();

                    let wants_targets = self.source.unwrap().wants_targets(db);

                    if targets.len() >= wants_targets || targets.len() >= self.valid_targets.len() {
                        vec![ActionResult::AddAbilityToStack {
                            source: self.source.unwrap(),
                            ability: *ability,
                            targets,
                        }]
                    } else {
                        vec![]
                    }
                }
            }
            UnresolvedActionResult::Attach(aura, card) => {
                if choice.is_none() && !self.optional && self.valid_targets.len() > 1 {
                    return vec![];
                } else if self.valid_targets.is_empty() {
                    return vec![ActionResult::PermanentToGraveyard(*card)];
                }

                vec![ActionResult::ApplyAuraToTarget {
                    aura: *aura,
                    target: choice
                        .map_or(self.valid_targets[0], |choice| self.valid_targets[choice]),
                }]
            }
            UnresolvedActionResult::AddCardToStack => {
                let mut targets = self
                    .choices
                    .iter()
                    .filter_map(|(target, count)| {
                        target.map(|target| std::iter::repeat(target).take(*count))
                    })
                    .flatten()
                    .map(|target| self.valid_targets[target])
                    .collect_vec();

                if targets.is_empty() && self.valid_targets.len() == 1 {
                    targets = self.valid_targets.clone();
                }

                let wants_targets = self.source.unwrap().wants_targets(db);
                if targets.len() >= wants_targets || targets.len() >= self.valid_targets.len() {
                    vec![ActionResult::AddCardToStack {
                        card: self.source.unwrap(),
                        targets,
                    }]
                } else {
                    vec![]
                }
            }
            UnresolvedActionResult::AddTriggerToStack(trigger) => {
                let mut targets = self
                    .choices
                    .iter()
                    .filter_map(|(target, count)| {
                        target.map(|target| std::iter::repeat(target).take(*count))
                    })
                    .flatten()
                    .map(|target| self.valid_targets[target])
                    .collect_vec();

                if targets.is_empty() && self.valid_targets.len() == 1 {
                    targets = self.valid_targets.clone();
                }

                let wants_targets = trigger.wants_targets(db, self.source.unwrap());
                if targets.len() >= wants_targets || targets.len() >= self.valid_targets.len() {
                    vec![ActionResult::AddTriggerToStack {
                        trigger: *trigger,
                        source: self.source.unwrap(),
                        targets,
                    }]
                } else {
                    vec![]
                }
            }
            UnresolvedActionResult::OrganizeStack(choices) => {
                if choice.is_none() {
                    debug!("Returning default entry order");
                    return vec![ActionResult::UpdateStackEntries(choices.clone())];
                }

                let targets = self
                    .choices
                    .iter()
                    .filter_map(|(target, count)| {
                        target.map(|target| std::iter::repeat(target).take(*count))
                    })
                    .flatten()
                    .collect_vec();

                if targets.len() < choices.len() {
                    return vec![];
                }

                debug!("Target order {:?}", targets);

                let mut results = vec![];
                for choice in targets {
                    results.push(choices[choice].clone());
                }

                vec![ActionResult::UpdateStackEntries(results)]
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    UpdateStackEntries(Vec<StackEntry>),
    PlayerLoses(Owner),
    RevealCard(CardId),
    MoveToHandFromLibrary(CardId),
    Shuffle(Owner),
    AddToBattlefield(CardId),
    AddToBattlefieldFromLibrary {
        card: CardId,
        enters_tapped: bool,
    },
    StackToGraveyard(CardId),
    ApplyToBattlefield(ModifierId),
    ApplyAuraToTarget {
        aura: AuraId,
        target: ActiveTarget,
    },
    ApplyModifierToTarget {
        modifier: ModifierId,
        target: ActiveTarget,
    },
    ExileTarget(ActiveTarget),
    DamageTarget {
        quantity: usize,
        target: ActiveTarget,
    },
    ManifestTopOfLibrary(Controller),
    ModifyCreatures {
        targets: Vec<ActiveTarget>,
        modifier: ModifierId,
    },
    SpellCountered {
        id: Entry,
    },
    AddCounters {
        target: CardId,
        counter: Counter,
        count: usize,
    },
    TapPermanent(CardId),
    PermanentToGraveyard(CardId),
    AddAbilityToStack {
        source: CardId,
        ability: AbilityId,
        targets: Vec<ActiveTarget>,
    },
    AddTriggerToStack {
        trigger: TriggerId,
        source: CardId,
        targets: Vec<ActiveTarget>,
    },
    CloneCreatureNonTargeting {
        source: CardId,
        target: Option<ActiveTarget>,
    },
    AddModifier {
        modifier: ModifierId,
    },
    Mill {
        count: usize,
        targets: Vec<ActiveTarget>,
    },
    ReturnFromGraveyardToLibrary {
        targets: Vec<ActiveTarget>,
    },
    ReturnFromGraveyardToBattlefield {
        targets: Vec<ActiveTarget>,
    },
    LoseLife {
        target: Controller,
        count: usize,
    },
    GainMana {
        gain: Vec<Mana>,
        target: Controller,
    },
    CreateToken {
        source: Controller,
        token: Token,
    },
    DrawCards {
        target: Controller,
        count: usize,
    },
    AddCardToStack {
        card: CardId,
        targets: Vec<ActiveTarget>,
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

#[derive(Debug)]
pub struct Battlefield;

impl Battlefield {
    pub fn is_empty(db: &mut Database) -> bool {
        db.query_filtered::<(), With<OnBattlefield>>()
            .iter(db)
            .next()
            .is_none()
    }

    pub fn no_modifiers(db: &mut Database) -> bool {
        db.modifiers
            .query_filtered::<Entity, With<Active>>()
            .iter(&db.modifiers)
            .next()
            .is_none()
    }

    pub fn number_of_cards_in_graveyard(db: &mut Database, player: Controller) -> usize {
        let mut query = db.query_filtered::<&Controller, With<InGraveyard>>();

        let mut count = 0;
        for controller in query.iter(db) {
            if player == *controller {
                count += 1
            }
        }

        count
    }

    pub fn creatures(db: &mut Database) -> Vec<CardId> {
        let on_battlefield = cards::<OnBattlefield>(db);

        let mut results = vec![];

        for card in on_battlefield.into_iter() {
            let types = card.types(db);
            if types.contains(&Type::Creature) {
                results.push(card);
            }
        }

        results
    }

    pub fn add_from_stack_or_hand(
        db: &mut Database,
        source_card_id: CardId,
        targets: Vec<CardId>,
    ) -> PendingResults {
        let mut results = PendingResults::default();

        ReplacementEffectId::activate_all_for_card(db, source_card_id);

        for replacement in ReplacementEffectId::watching::<replacing::Etb>(db) {
            let source = replacement.source(db);
            let controller = replacement.source(db).controller(db);
            let restrictions = replacement.restrictions(db);
            if !source.passes_restrictions(
                db,
                source,
                controller,
                ControllerRestriction::Any,
                &restrictions,
            ) {
                continue;
            }

            for effect in replacement.effects(db) {
                let effect = effect.into_effect(db, controller);
                Self::push_effect_results(db, source, controller, effect, &mut results);
            }
        }

        if let Some(aura) = source_card_id.aura(db) {
            for target in targets.iter() {
                target.apply_aura(db, aura);
            }
        }

        for ability in source_card_id.static_abilities(db) {
            match ability {
                StaticAbility::GreenCannotBeCountered { .. } => {}
                StaticAbility::BattlefieldModifier(modifier) => {
                    let modifier =
                        ModifierId::upload_temporary_modifier(db, source_card_id, &modifier);
                    results.push_resolved(ActionResult::AddModifier { modifier })
                }
                StaticAbility::ExtraLandsPerTurn(_) => {}
            }
        }

        for ability in source_card_id.etb_abilities(db) {
            results.extend(Stack::move_ability_to_stack(db, ability, source_card_id));
        }

        if source_card_id.etb_tapped(db) {
            source_card_id.tap(db);
        }
        source_card_id.move_to_battlefield(db);

        for trigger in
            TriggerId::active_triggers_of_source::<trigger_source::EntersTheBattlefield>(db)
        {
            if matches!(trigger.location_from(db), triggers::Location::Anywhere) {
                let for_types = trigger.for_types(db);
                if source_card_id.types_intersect(db, &for_types) {
                    for listener in trigger.listeners(db) {
                        results.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                    }
                }
            }
        }

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        results
    }

    pub fn add_from_library(
        db: &mut Database,
        source_card_id: CardId,
        enters_tapped: bool,
    ) -> PendingResults {
        let mut results = PendingResults::default();

        for replacement in ReplacementEffectId::watching::<replacing::Etb>(db) {
            let source = replacement.source(db);
            let controller = replacement.source(db).controller(db);
            let restrictions = replacement.restrictions(db);
            if !source.passes_restrictions(
                db,
                source,
                controller,
                ControllerRestriction::Any,
                &restrictions,
            ) {
                continue;
            }

            for effect in replacement.effects(db) {
                let effect = effect.into_effect(db, controller);
                Self::push_effect_results(db, source, controller, effect, &mut results);
            }
        }

        if let Some(aura) = source_card_id.aura(db) {
            results.push_unresolved(UnresolvedAction {
                source: Some(source_card_id),
                result: UnresolvedActionResult::Attach(aura, source_card_id),
                valid_targets: source_card_id.valid_targets(db),
                choices: Default::default(),
                optional: false,
            });
        }

        for ability in source_card_id.static_abilities(db) {
            match ability {
                StaticAbility::GreenCannotBeCountered { .. } => {}
                StaticAbility::BattlefieldModifier(modifier) => {
                    let modifier =
                        ModifierId::upload_temporary_modifier(db, source_card_id, &modifier);
                    results.push_resolved(ActionResult::AddModifier { modifier })
                }
                StaticAbility::ExtraLandsPerTurn(_) => {}
            }
        }

        for ability in source_card_id.etb_abilities(db) {
            results.extend(Stack::move_ability_to_stack(db, ability, source_card_id));
        }

        if enters_tapped || source_card_id.etb_tapped(db) {
            source_card_id.tap(db);
        }
        source_card_id.move_to_battlefield(db);

        for trigger in
            TriggerId::active_triggers_of_source::<trigger_source::EntersTheBattlefield>(db)
        {
            if matches!(
                trigger.location_from(db),
                triggers::Location::Anywhere | triggers::Location::Library
            ) {
                let for_types = trigger.for_types(db);
                if source_card_id.types_intersect(db, &for_types) {
                    for listener in trigger.listeners(db) {
                        results.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                    }
                }
            }
        }

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        results
    }

    pub fn controlled_colors(db: &mut Database, player: Controller) -> HashSet<Color> {
        let cards = player.get_cards::<OnBattlefield>(db);

        let mut colors = HashSet::default();
        for card in cards {
            let card_colors = card.colors(db);
            colors.extend(card_colors)
        }

        colors
    }

    pub fn untap(db: &mut Database, player: Owner) {
        let cards = player.get_cards::<OnBattlefield>(db);
        for card in cards {
            card.untap(db);
        }
    }

    pub fn end_turn(db: &mut Database) {
        let cards = cards::<OnBattlefield>(db);
        for card in cards {
            card.clear_damage(db);
        }

        let all_modifiers = db
            .modifiers
            .query_filtered::<Entity, (With<Active>, With<UntilEndOfTurn>)>()
            .iter(&db.modifiers)
            .map(ModifierId::from)
            .collect_vec();

        for modifier in all_modifiers {
            modifier.detach_all(db);
        }

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }
    }

    pub fn check_sba(db: &mut Database) -> Vec<ActionResult> {
        let mut result = vec![];
        for card_id in cards::<OnBattlefield>(db) {
            let toughness = card_id.toughness(db);

            if toughness.is_some() && (toughness.unwrap() - card_id.marked_damage(db)) <= 0 {
                result.push(ActionResult::PermanentToGraveyard(card_id));
            }

            let aura = card_id.aura(db);
            if aura.is_some() && !aura.unwrap().is_attached(db) {
                result.push(ActionResult::PermanentToGraveyard(card_id));
            }
        }

        result
    }

    pub fn activate_ability(
        db: &mut Database,
        all_players: &mut AllPlayers,
        card: CardId,
        index: usize,
    ) -> PendingResults {
        if Stack::split_second(db) {
            return PendingResults::default();
        }

        let mut results = PendingResults::default();

        let ability_id = card.activated_abilities(db)[index];
        let ability = ability_id.ability(db);

        if let Some(cost) = ability.cost() {
            if cost.tap {
                if card.tapped(db) {
                    return PendingResults::default();
                }

                results.push_resolved(ActionResult::TapPermanent(card));
            }

            for cost in cost.additional_cost.iter() {
                match cost {
                    AdditionalCost::SacrificeThis => {
                        if !card.can_be_sacrificed(db) {
                            return PendingResults::default();
                        }

                        results.push_resolved(ActionResult::PermanentToGraveyard(card));
                    }
                }
            }

            if !all_players[card.controller(db)].spend_mana(&cost.mana_cost) {
                return PendingResults::default();
            }
        }

        if let Ability::Mana(gain) = ability {
            match gain.gain {
                GainMana::Specific { gains } => {
                    results.push_resolved(ActionResult::GainMana {
                        target: card.controller(db),
                        gain: gains,
                    });
                }
                GainMana::Choice { .. } => {
                    results.push_unresolved(UnresolvedAction {
                        source: Some(card),
                        result: UnresolvedActionResult::Ability(ability_id),
                        optional: false,
                        valid_targets: vec![],
                        choices: Default::default(),
                    });
                }
            }
        } else {
            let controller = card.controller(db);

            let creatures = Self::creatures(db);
            let mut valid_targets = vec![];
            for effect in ability.into_effects() {
                let effect = effect.into_effect(db, controller);
                card.targets_for_effect(db, controller, &effect, &creatures, &mut valid_targets);
            }

            results.push_unresolved(UnresolvedAction {
                source: Some(card),
                result: UnresolvedActionResult::Ability(ability_id),
                // TODO this isn't always true for many abilities.
                optional: false,
                valid_targets,
                choices: Default::default(),
            });
        }

        results
    }

    pub fn static_abilities(db: &mut Database) -> Vec<(StaticAbility, Controller)> {
        let mut result: Vec<(StaticAbility, Controller)> = Default::default();

        for card in cards::<OnBattlefield>(db) {
            let controller = card.controller(db);
            for ability in card.static_abilities(db) {
                result.push((ability, controller));
            }
        }

        result
    }

    pub fn apply_action_results(
        db: &mut Database,
        all_players: &mut AllPlayers,
        results: &[ActionResult],
    ) -> PendingResults {
        let mut pending = PendingResults::default();

        for result in results.iter() {
            pending.extend(Self::apply_action_result(db, all_players, result));
        }

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        pending
    }

    fn apply_action_result(
        db: &mut Database,
        all_players: &mut AllPlayers,
        result: &ActionResult,
    ) -> PendingResults {
        match result {
            ActionResult::TapPermanent(card_id) => {
                card_id.tap(db);
            }
            ActionResult::PermanentToGraveyard(card_id) => {
                return Self::permanent_to_graveyard(db, *card_id);
            }
            ActionResult::AddAbilityToStack {
                source,
                ability,
                targets,
            } => {
                ability.move_to_stack(db, *source, targets.clone());
            }
            ActionResult::AddTriggerToStack {
                trigger,
                source,
                targets,
            } => {
                trigger.move_to_stack(db, *source, targets.clone());
            }
            ActionResult::CloneCreatureNonTargeting { source, target } => {
                if let Some(ActiveTarget::Battlefield { id: target }) = target {
                    source.clone_card(db, *target);
                }
            }
            ActionResult::AddModifier { modifier } => {
                modifier.activate(db);
            }
            ActionResult::Mill { count, targets } => {
                let mut pending = PendingResults::default();
                for target in targets {
                    let ActiveTarget::Player { id: target } = target else {
                        unreachable!()
                    };

                    let deck = &mut all_players[*target].deck;
                    for _ in 0..*count {
                        let card_id = deck.draw();
                        if let Some(card_id) = card_id {
                            pending.extend(Self::library_to_graveyard(db, card_id));
                        }
                    }
                }

                return pending;
            }
            ActionResult::ReturnFromGraveyardToLibrary { targets } => {
                for target in targets {
                    let ActiveTarget::Graveyard { id: target } = target else {
                        unreachable!()
                    };

                    all_players[target.owner(db)].deck.place_on_top(db, *target);
                }
            }
            ActionResult::ReturnFromGraveyardToBattlefield { targets } => {
                let mut pending = PendingResults::default();
                for target in targets {
                    let ActiveTarget::Graveyard { id: target } = target else {
                        unreachable!()
                    };
                    pending.extend(Self::add_from_stack_or_hand(db, *target, vec![]));
                }

                return pending;
            }
            ActionResult::LoseLife { target, count } => {
                all_players[*target].life_total -= *count as i32;
            }
            ActionResult::GainMana { gain, target } => {
                for mana in gain {
                    all_players[*target].mana_pool.apply(*mana)
                }
            }
            ActionResult::CreateToken { source, token } => {
                let card = CardId::upload_token(db, (*source).into(), token.clone());
                return Battlefield::add_from_stack_or_hand(db, card, vec![]);
            }
            ActionResult::DrawCards { target, count } => {
                let _ = all_players[*target].draw(db, *count);
            }
            ActionResult::AddToBattlefield(card) => {
                return Battlefield::add_from_stack_or_hand(db, *card, vec![]);
            }
            ActionResult::StackToGraveyard(card) => {
                return Battlefield::stack_to_graveyard(db, *card);
            }
            ActionResult::ApplyToBattlefield(modifier) => {
                modifier.activate(db);
            }
            ActionResult::ApplyModifierToTarget { modifier, target } => {
                let ActiveTarget::Battlefield { id: target } = target else {
                    unreachable!()
                };
                target.apply_modifier(db, *modifier);
            }
            ActionResult::ExileTarget(target) => {
                let ActiveTarget::Battlefield { id: target } = target else {
                    unreachable!()
                };
                return Battlefield::exile(db, *target);
            }
            ActionResult::DamageTarget { quantity, target } => {
                let ActiveTarget::Battlefield { id: target } = target else {
                    unreachable!()
                };
                target.mark_damage(db, *quantity);
            }
            ActionResult::ManifestTopOfLibrary(player) => {
                return all_players[*player].manifest(db);
            }
            ActionResult::ModifyCreatures { targets, modifier } => {
                for target in targets {
                    let ActiveTarget::Battlefield { id: target } = target else {
                        unreachable!()
                    };
                    target.apply_modifier(db, *modifier);
                }
            }
            ActionResult::SpellCountered { id } => match id {
                Entry::Card(card) => {
                    return Battlefield::stack_to_graveyard(db, *card);
                }
                Entry::Ability { .. } | Entry::Trigger { .. } => unreachable!(),
            },
            ActionResult::AddCounters {
                target,
                counter,
                count,
            } => {
                CounterId::add_counters(db, *target, *counter, *count);
            }
            ActionResult::RevealCard(card) => {
                card.reveal(db);
            }
            ActionResult::MoveToHandFromLibrary(card) => {
                card.move_to_hand(db);
                all_players[card.controller(db)].deck.remove(*card);
            }
            ActionResult::AddToBattlefieldFromLibrary {
                card,
                enters_tapped,
            } => {
                all_players[card.controller(db)].deck.remove(*card);
                return Battlefield::add_from_library(db, *card, *enters_tapped);
            }
            ActionResult::Shuffle(owner) => {
                all_players[*owner].deck.shuffle();
            }
            ActionResult::ApplyAuraToTarget { aura, target } => {
                match target {
                    ActiveTarget::Battlefield { id } => {
                        id.apply_aura(db, *aura);
                    }
                    ActiveTarget::Graveyard { .. } => todo!(),
                    ActiveTarget::Player { .. } => todo!(),
                    _ => unreachable!(),
                };
            }
            ActionResult::PlayerLoses(player) => {
                all_players[*player].lost = true;
            }
            ActionResult::AddCardToStack { card, targets } => {
                card.move_to_stack(db, targets.clone());
            }
            ActionResult::UpdateStackEntries(entries) => {
                for entry in entries.iter() {
                    match entry.ty {
                        Entry::Card(card) => {
                            card.move_to_stack(db, entry.targets.clone());
                        }
                        Entry::Ability { in_stack, .. } => {
                            in_stack.update_stack_seq(db);
                        }
                        Entry::Trigger { in_stack, .. } => {
                            in_stack.update_stack_seq(db);
                        }
                    }
                }
            }
        }

        PendingResults::default()
    }

    pub fn permanent_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(
                trigger.location_from(db),
                triggers::Location::Anywhere | triggers::Location::Battlefield
            ) {
                let for_types = trigger.for_types(db);
                if target.types_intersect(db, &for_types) {
                    for listener in trigger.listeners(db) {
                        pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                    }
                }
            }
        }

        target.move_to_graveyard(db);

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        pending
    }

    pub fn library_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(
                trigger.location_from(db),
                triggers::Location::Anywhere | triggers::Location::Library
            ) {
                let for_types = trigger.for_types(db);
                if target.types_intersect(db, &for_types) {
                    for listener in trigger.listeners(db) {
                        pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                    }
                }
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    pub fn stack_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(trigger.location_from(db), triggers::Location::Anywhere) {
                let for_types = trigger.for_types(db);
                if target.types_intersect(db, &for_types) {
                    for listener in trigger.listeners(db) {
                        pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                    }
                }
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    pub fn exile(db: &mut Database, target: CardId) -> PendingResults {
        target.move_to_exile(db);

        PendingResults::default()
    }

    fn push_effect_results(
        db: &mut Database,
        source: CardId,
        controller: Controller,
        effect: Effect,
        results: &mut PendingResults,
    ) {
        match effect {
            Effect::BattlefieldModifier(modifier) => {
                results.push_resolved(ActionResult::AddModifier {
                    modifier: ModifierId::upload_temporary_modifier(db, source, &modifier),
                });
            }
            Effect::ControllerDrawCards(count) => {
                results.push_resolved(ActionResult::DrawCards {
                    target: controller,
                    count,
                });
            }
            Effect::ControllerLosesLife(count) => {
                results.push_resolved(ActionResult::LoseLife {
                    target: controller,
                    count,
                });
            }
            Effect::CreateToken(token) => {
                results.push_resolved(ActionResult::CreateToken {
                    source: controller,
                    token,
                });
            }
            Effect::GainCounter(counter) => {
                results.push_resolved(ActionResult::AddCounters {
                    target: source,
                    counter,
                    count: 1,
                });
            }
            Effect::CopyOfAnyCreatureNonTargeting => {
                let creatures = Self::creatures(db);
                let mut valid_targets = vec![];
                source.targets_for_effect(db, controller, &effect, &creatures, &mut valid_targets);

                results.push_unresolved(UnresolvedAction {
                    source: Some(source),
                    result: UnresolvedActionResult::Effect(effect),
                    optional: true,
                    valid_targets,
                    choices: Default::default(),
                })
            }
            Effect::TutorLibrary(_) => {
                let creatures = Self::creatures(db);
                let mut valid_targets = vec![];
                source.targets_for_effect(db, controller, &effect, &creatures, &mut valid_targets);

                results.push_unresolved(UnresolvedAction {
                    source: Some(source),
                    result: UnresolvedActionResult::Effect(effect),
                    // TODO this isn't always true
                    optional: true,
                    valid_targets,
                    choices: Default::default(),
                })
            }
            Effect::CounterSpell { .. }
            | Effect::DealDamage(_)
            | Effect::Equip(_)
            | Effect::ExileTargetCreature
            | Effect::ExileTargetCreatureManifestTopOfLibrary
            | Effect::Mill(_)
            | Effect::ModifyCreature(_)
            | Effect::ReturnFromGraveyardToBattlefield(_)
            | Effect::ReturnFromGraveyardToLibrary(_) => {
                let creatures = Self::creatures(db);
                let mut valid_targets = vec![];
                source.targets_for_effect(db, controller, &effect, &creatures, &mut valid_targets);

                results.push_unresolved(UnresolvedAction {
                    source: Some(source),
                    result: UnresolvedActionResult::Effect(effect),
                    // TODO this isn't always true for many effects.
                    optional: false,
                    valid_targets,
                    choices: Default::default(),
                })
            }
        }
    }
}

pub fn compute_deck_targets(
    db: &mut Database,
    player: Controller,
    restrictions: &[Restriction],
) -> Vec<CardId> {
    let mut results = vec![];

    for card in player.get_cards::<InLibrary>(db) {
        if !card.passes_restrictions(db, card, player, ControllerRestriction::You, restrictions) {
            continue;
        }

        results.push(card);
    }

    results
}

pub fn compute_graveyard_targets(
    db: &mut Database,
    controller: ControllerRestriction,
    source_card: CardId,
    types: &HashSet<Type>,
) -> Vec<CardId> {
    let targets = match controller {
        ControllerRestriction::Any => AllPlayers::all_players_in_db(db),
        ControllerRestriction::You => HashSet::from([source_card.controller(db).into()]),
        ControllerRestriction::Opponent => {
            let mut all = AllPlayers::all_players_in_db(db);
            all.remove(&source_card.controller(db).into());
            all
        }
    };
    let mut target_cards = vec![];

    for target in targets.into_iter() {
        let cards_in_graveyard = target.get_cards::<InGraveyard>(db);
        for card in cards_in_graveyard {
            if card.types_intersect(db, types) {
                target_cards.push(card);
            }
        }
    }

    target_cards
}
