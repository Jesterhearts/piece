use std::collections::VecDeque;

use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    abilities::{Ability, GainMana, GainManaAbility},
    battlefield::{ActionResult, Battlefield},
    controller::ControllerRestriction,
    effects::{BattlefieldModifier, Destination, Effect, EffectDuration, Mill, TutorLibrary},
    in_play::{AbilityId, AuraId, CardId, Database, ModifierId},
    player::AllPlayers,
    stack::{ActiveTarget, StackEntry},
};

#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum ResolutionResult {
    Complete,
    TryAgain,
    PendingChoice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingResult {
    pub apply_immediately: Vec<ActionResult>,
    pub to_resolve: VecDeque<UnresolvedAction>,
    pub then_apply: Vec<ActionResult>,
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PendingResults {
    pub results: VecDeque<PendingResult>,
    pub applied: bool,
}

impl<const T: usize> From<[ActionResult; T]> for PendingResults {
    fn from(value: [ActionResult; T]) -> Self {
        Self {
            results: VecDeque::from([PendingResult {
                apply_immediately: value.to_vec(),
                to_resolve: Default::default(),
                then_apply: vec![],
            }]),
            applied: false,
        }
    }
}

impl PendingResults {
    pub fn clear(&mut self) -> bool {
        if self.applied {
            false
        } else {
            self.results.clear();
            true
        }
    }

    pub fn push_resolved(&mut self, action: ActionResult) {
        if let Some(last) = self.results.back_mut() {
            if !last.to_resolve.is_empty() || !last.then_apply.is_empty() {
                self.results.push_back(PendingResult {
                    apply_immediately: vec![action],
                    to_resolve: Default::default(),
                    then_apply: vec![],
                });
            } else {
                last.apply_immediately.push(action);
            }
        } else {
            self.results.push_back(PendingResult {
                apply_immediately: vec![action],
                to_resolve: Default::default(),
                then_apply: vec![],
            });
        }
    }

    pub fn push_deferred(&mut self, action: ActionResult) {
        if let Some(last) = self.results.back_mut() {
            if !last.to_resolve.is_empty() {
                self.results.push_back(PendingResult {
                    apply_immediately: vec![],
                    to_resolve: Default::default(),
                    then_apply: vec![action],
                });
            } else {
                last.then_apply.push(action);
            }
        } else {
            self.results.push_back(PendingResult {
                apply_immediately: vec![],
                to_resolve: Default::default(),
                then_apply: vec![action],
            });
        }
    }

    pub fn push_unresolved(&mut self, action: UnresolvedAction) {
        if let Some(last) = self.results.back_mut() {
            if !last.then_apply.is_empty() {
                self.results.push_back(PendingResult {
                    apply_immediately: Default::default(),
                    to_resolve: VecDeque::from([action]),
                    then_apply: vec![],
                });
            } else {
                last.to_resolve.push_back(action);
            }
        } else {
            self.results.push_back(PendingResult {
                apply_immediately: Default::default(),
                to_resolve: VecDeque::from([action]),
                then_apply: vec![],
            });
        }
    }

    pub fn is_optional(&self, db: &mut Database) -> bool {
        if let Some(to_resolve) = self.results.front() {
            if let Some(to_resolve) = to_resolve.to_resolve.front() {
                to_resolve.optional
                    || (to_resolve.source.is_some()
                        && to_resolve
                            .result
                            .wants_targets(db, to_resolve.source.unwrap())
                            .into_iter()
                            .enumerate()
                            .all(|(idx, wants)| {
                                to_resolve
                                    .choices
                                    .get(idx + 1)
                                    .map(|choices| choices.len())
                                    .unwrap_or_default()
                                    >= wants
                            })
                        || to_resolve
                            .result
                            .wants_targets(db, to_resolve.source.unwrap())
                            .into_iter()
                            .zip(to_resolve.valid_targets.iter())
                            .all(|(wants, targets)| wants > targets.len()))
            } else {
                true
            }
        } else {
            true
        }
    }

    pub fn options(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        if let Some(to_resolve) = self.results.front() {
            if let Some(to_resolve) = to_resolve.to_resolve.front() {
                to_resolve.choices(db, all_players)
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    pub fn description(&self, db: &mut Database) -> String {
        if let Some(to_resolve) = self.results.front() {
            if let Some(to_resolve) = to_resolve.to_resolve.front() {
                to_resolve.description(db)
            } else {
                String::default()
            }
        } else {
            String::default()
        }
    }

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
        if !first.apply_immediately.is_empty() {
            self.applied = true;
        }
        let immediate_results =
            Battlefield::apply_action_results(db, all_players, &first.apply_immediately);
        first.apply_immediately.clear();

        let from_resolution = if let Some(to_resolve) = first.to_resolve.front_mut() {
            to_resolve.compute_targets(db);
            let actions = to_resolve.resolve(db, choice);
            if !actions.is_empty() {
                self.applied = true;
                first.to_resolve.pop_front();
                Battlefield::apply_action_results(db, all_players, &actions)
            } else {
                return ResolutionResult::PendingChoice;
            }
        } else {
            PendingResults::default()
        };

        let after = if first.to_resolve.is_empty() {
            if !first.then_apply.is_empty() {
                self.applied = true;
            }

            let results = Battlefield::apply_action_results(db, all_players, &first.then_apply);

            self.results.pop_front();
            results
        } else {
            PendingResults::default()
        };

        self.extend(immediate_results);
        self.extend(from_resolution);
        self.extend(after);

        if self.results.is_empty() {
            ResolutionResult::Complete
        } else {
            ResolutionResult::TryAgain
        }
    }

    pub fn extend(&mut self, results: PendingResults) {
        self.results.extend(results.results);
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn only_immediate_results(&self) -> bool {
        self.is_empty()
            || (self.results.len() == 1
                && (self.results.front().unwrap().to_resolve.is_empty()
                    || (self.results.front().unwrap().to_resolve.len() == 1
                        && self
                            .results
                            .front()
                            .unwrap()
                            .to_resolve
                            .front()
                            .unwrap()
                            .valid_targets
                            .is_empty())))
            || self
                .results
                .iter()
                .all(|results| results.to_resolve.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedActionResult {
    Effect(Effect),
    Attach(AuraId),
    Ability { ability: AbilityId, choosing: usize },
    AddCardToStack { choosing: usize },
    OrganizeStack(Vec<StackEntry>),
    SacrificePermanent,
}

impl UnresolvedActionResult {
    fn wants_targets(&self, db: &mut Database, source: CardId) -> Vec<usize> {
        match self {
            UnresolvedActionResult::Effect(effect) => vec![effect.wants_targets()],
            UnresolvedActionResult::Attach(_) => vec![1],
            UnresolvedActionResult::Ability { ability, .. } => {
                let effects = ability.effects(db);
                let controller = ability.controller(db);
                effects
                    .into_iter()
                    .map(|effect| effect.into_effect(db, controller))
                    .map(|effect| effect.wants_targets())
                    .collect_vec()
            }
            UnresolvedActionResult::AddCardToStack { .. } => source.wants_targets(db),
            UnresolvedActionResult::OrganizeStack(_) => vec![0],
            UnresolvedActionResult::SacrificePermanent => vec![1],
        }
    }

    fn choice_sets(&self, db: &mut Database, source: Option<CardId>) -> usize {
        match self {
            UnresolvedActionResult::Effect(_) => 1,
            UnresolvedActionResult::Attach(_) => 1,
            UnresolvedActionResult::Ability { ability, .. } => ability.effects(db).len(),
            UnresolvedActionResult::AddCardToStack { .. } => source.unwrap().effects(db).len(),
            UnresolvedActionResult::OrganizeStack(_) => 1,
            UnresolvedActionResult::SacrificePermanent => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedAction {
    source: Option<CardId>,
    result: UnresolvedActionResult,
    valid_targets: Vec<Vec<ActiveTarget>>,
    choices: Vec<IndexMap<Option<usize>, usize>>,
    optional: bool,
}

impl UnresolvedAction {
    pub fn new(
        db: &mut Database,
        source: Option<CardId>,
        result: UnresolvedActionResult,
        valid_targets: Vec<Vec<ActiveTarget>>,
        optional: bool,
    ) -> Self {
        let choice_sets = result.choice_sets(db, source);

        Self {
            source,
            result,
            valid_targets,
            choices: vec![Default::default(); choice_sets],
            optional,
        }
    }

    pub fn compute_targets(&mut self, db: &mut Database) {
        if let Some(source) = self.source {
            let controller = source.controller(db);
            match &self.result {
                UnresolvedActionResult::Effect(effect) => {
                    let creatures = Battlefield::creatures(db);

                    let valid_targets =
                        vec![source.targets_for_effect(db, controller, effect, &creatures)];

                    self.valid_targets = valid_targets;
                }
                UnresolvedActionResult::Ability { ability, .. } => {
                    let ability = ability.ability(db);
                    let creatures = Battlefield::creatures(db);

                    let mut valid_targets = vec![];
                    for effect in ability.into_effects() {
                        let effect = effect.into_effect(db, controller);

                        valid_targets
                            .push(source.targets_for_effect(db, controller, &effect, &creatures));
                    }

                    self.valid_targets = valid_targets;
                }
                UnresolvedActionResult::Attach(_) => {
                    self.valid_targets = self.source.unwrap().valid_targets(db);
                }
                UnresolvedActionResult::AddCardToStack { .. } => {
                    let creatures = Battlefield::creatures(db);

                    let mut valid_targets = vec![];
                    for effect in source.effects(db) {
                        let effect = effect.into_effect(db, controller);

                        valid_targets
                            .push(source.targets_for_effect(db, controller, &effect, &creatures));
                    }

                    self.valid_targets = valid_targets;
                }
                UnresolvedActionResult::OrganizeStack(_) => {}
                UnresolvedActionResult::SacrificePermanent => {}
            }
        }
    }

    pub fn description(&self, db: &mut Database) -> String {
        match &self.result {
            UnresolvedActionResult::Effect(_) => "Effect".to_string(),
            UnresolvedActionResult::Attach(_) => "Aura".to_string(),
            UnresolvedActionResult::Ability { ability, .. } => ability.text(db),
            UnresolvedActionResult::AddCardToStack { .. } => self.source.unwrap().name(db),
            UnresolvedActionResult::OrganizeStack(_) => "stack order".to_string(),
            UnresolvedActionResult::SacrificePermanent => "sacrificing a permanent".to_string(),
        }
    }

    pub fn choices(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        match &self.result {
            UnresolvedActionResult::Effect(effect) => effect
                .choices(db, all_players, &self.valid_targets[0])
                .into_iter()
                .enumerate()
                .filter(|(idx, _)| !self.choices[0].contains_key(&Some(*idx)))
                .collect_vec(),
            UnresolvedActionResult::Ability { ability, choosing } => {
                let controller = self.source.unwrap().controller(db);
                ability
                    .ability(db)
                    .choices(
                        db,
                        all_players,
                        controller,
                        &self.valid_targets[*choosing],
                        *choosing,
                    )
                    .into_iter()
                    .enumerate()
                    .filter(|(idx, _)| !self.choices[*choosing].contains_key(&Some(*idx)))
                    .collect_vec()
            }
            UnresolvedActionResult::AddCardToStack { choosing } => self.valid_targets[*choosing]
                .iter()
                .map(|target| target.display(db, all_players))
                .enumerate()
                .filter(|(idx, _)| !self.choices[*choosing].contains_key(&Some(*idx)))
                .collect_vec(),
            UnresolvedActionResult::Attach(_) | UnresolvedActionResult::SacrificePermanent => self
                .valid_targets[0]
                .iter()
                .map(|target| target.display(db, all_players))
                .enumerate()
                .filter(|(idx, _)| !self.choices[0].contains_key(&Some(*idx)))
                .collect_vec(),
            UnresolvedActionResult::OrganizeStack(entries) => entries
                .iter()
                .map(|e| e.display(db))
                .enumerate()
                .filter(|(idx, _)| !self.choices[0].contains_key(&Some(*idx)))
                .collect_vec(),
        }
    }

    pub fn resolve(&mut self, db: &mut Database, choice: Option<usize>) -> Vec<ActionResult> {
        match &mut self.result {
            UnresolvedActionResult::Effect(effect) => {
                *self.choices[0].entry(choice).or_default() += 1;
                let valid_targets = &self.valid_targets[0];
                let choices = &self.choices[0];
                let wants_targets = effect.wants_targets();

                match effect {
                    Effect::CopyOfAnyCreatureNonTargeting => {
                        vec![ActionResult::CloneCreatureNonTargeting {
                            source: self.source.unwrap(),
                            target: choice.map(|choice| valid_targets[choice]),
                        }]
                    }
                    Effect::CounterSpell { .. } => {
                        todo!()
                    }
                    Effect::CreateToken(token) => vec![ActionResult::CreateToken {
                        source: self.source.unwrap().controller(db),
                        token: token.clone(),
                    }],
                    Effect::DealDamage(damage) => {
                        debug!("Choice: {:?}", choice);
                        if choice.is_none() && !self.optional && valid_targets.len() > 1 {
                            return vec![];
                        }

                        vec![ActionResult::DamageTarget {
                            quantity: damage.quantity,
                            target: choice
                                .map_or_else(|| valid_targets[0], |choice| valid_targets[choice]),
                        }]
                    }
                    Effect::Equip(modifiers) => {
                        if choice.is_none() && !self.optional && valid_targets.len() > 1 {
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
                                    .map_or(valid_targets[0], |choice| valid_targets[choice]),
                            })
                        }

                        results
                    }
                    Effect::ExileTargetCreature => {
                        if choice.is_none() && !self.optional && valid_targets.len() > 1 {
                            return vec![];
                        }

                        vec![ActionResult::ExileTarget(choice.map_or_else(
                            || valid_targets[0],
                            |choice| valid_targets[choice],
                        ))]
                    }
                    Effect::ExileTargetCreatureManifestTopOfLibrary => {
                        if choice.is_none() && !self.optional && valid_targets.len() > 1 {
                            return vec![];
                        }

                        let target =
                            choice.map_or_else(|| valid_targets[0], |choice| valid_targets[choice]);

                        let ActiveTarget::Battlefield { id } = target else {
                            unreachable!();
                        };

                        vec![
                            ActionResult::ExileTarget(target),
                            ActionResult::ManifestTopOfLibrary(id.controller(db)),
                        ]
                    }
                    Effect::GainCounter(counter) => {
                        if choice.is_none() && !self.optional && valid_targets.len() > 1 {
                            return vec![];
                        }

                        let target =
                            choice.map_or_else(|| valid_targets[0], |choice| valid_targets[choice]);

                        let ActiveTarget::Battlefield { id } = target else {
                            unreachable!();
                        };
                        vec![ActionResult::AddCounters {
                            target: id,
                            counter: *counter,
                            count: 1,
                        }]
                    }
                    Effect::Mill(Mill { count, .. }) => {
                        let mut targets = choices
                            .iter()
                            .filter_map(|(target, count)| {
                                target.map(|target| std::iter::repeat(target).take(*count))
                            })
                            .flatten()
                            .map(|target| valid_targets[target])
                            .collect_vec();

                        if targets.is_empty() && valid_targets.len() == 1 {
                            targets = valid_targets.clone();
                        }

                        if targets.len() >= wants_targets || targets.len() >= valid_targets.len() {
                            vec![ActionResult::Mill {
                                count: *count,
                                targets,
                            }]
                        } else {
                            vec![]
                        }
                    }
                    Effect::ModifyCreature(modifier) => {
                        if choice.is_none() && !self.optional && valid_targets.len() > 1 {
                            return vec![];
                        }

                        let target =
                            choice.map_or_else(|| valid_targets[0], |choice| valid_targets[choice]);

                        let modifier = ModifierId::upload_temporary_modifier(
                            db,
                            self.source.unwrap(),
                            modifier,
                        );

                        vec![ActionResult::ModifyCreatures {
                            targets: vec![target],
                            modifier,
                        }]
                    }
                    Effect::ReturnFromGraveyardToBattlefield(_) => {
                        let mut targets = choices
                            .iter()
                            .filter_map(|(target, count)| {
                                target.map(|target| std::iter::repeat(target).take(*count))
                            })
                            .flatten()
                            .map(|target| valid_targets[target])
                            .collect_vec();

                        if targets.is_empty() && valid_targets.len() == 1 {
                            targets = valid_targets.clone();
                        }

                        if targets.len() >= wants_targets || targets.len() >= valid_targets.len() {
                            vec![ActionResult::ReturnFromGraveyardToBattlefield { targets }]
                        } else {
                            vec![]
                        }
                    }
                    Effect::ReturnFromGraveyardToLibrary(_) => {
                        let mut targets = choices
                            .iter()
                            .filter_map(|(target, count)| {
                                target.map(|target| std::iter::repeat(target).take(*count))
                            })
                            .flatten()
                            .map(|target| valid_targets[target])
                            .collect_vec();

                        if targets.is_empty() && valid_targets.len() == 1 {
                            targets = valid_targets.clone();
                        }

                        if targets.len() >= wants_targets || targets.len() >= valid_targets.len() {
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
                        let mut targets = choices
                            .iter()
                            .filter_map(|(target, count)| {
                                target.map(|target| std::iter::repeat(target).take(*count))
                            })
                            .flatten()
                            .map(|target| valid_targets[target])
                            .collect_vec();

                        if targets.is_empty() && valid_targets.len() == 1 {
                            targets = valid_targets.clone();
                        }

                        if targets.len() >= wants_targets || targets.len() >= valid_targets.len() {
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
                                Destination::TopOfLibrary => {
                                    for card in targets.iter() {
                                        results.push(ActionResult::MoveFromLibraryToTopOfLibrary(
                                            *card,
                                        ));
                                    }
                                }
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
                    Effect::CreateTokenCopy { modifiers } => {
                        if choice.is_none() && !self.optional && valid_targets.len() > 1 {
                            return vec![];
                        }

                        let target =
                            choice.map_or_else(|| valid_targets[0], |choice| valid_targets[choice]);

                        let ActiveTarget::Battlefield { id } = target else {
                            unreachable!();
                        };

                        vec![ActionResult::CreateTokenCopyOf {
                            target: id,
                            modifiers: modifiers.clone(),
                            controller: self.source.unwrap().controller(db),
                        }]
                    }

                    Effect::BattlefieldModifier(_)
                    | Effect::ControllerDrawCards(_)
                    | Effect::ControllerLosesLife(_)
                    | Effect::RevealEachTopOfLibrary(_)
                    | Effect::ReturnSelfToHand => {
                        unreachable!()
                    }
                }
            }
            UnresolvedActionResult::Ability { ability, choosing } => {
                if let Ability::Mana(GainManaAbility { gain, .. }) = ability.ability(db) {
                    match gain {
                        GainMana::Specific { gains } => {
                            vec![ActionResult::GainMana {
                                gain: gains,
                                target: self.source.unwrap().controller(db),
                            }]
                        }
                        GainMana::Choice { choices } => {
                            if choice.is_none() {
                                return vec![];
                            }

                            vec![ActionResult::GainMana {
                                gain: choices[choice.unwrap()].clone(),
                                target: self.source.unwrap().controller(db),
                            }]
                        }
                    }
                } else {
                    *self.choices[*choosing].entry(choice).or_default() += 1;
                    let targets = self.choices[*choosing]
                        .iter()
                        .filter_map(|(target, count)| {
                            target.map(|target| std::iter::repeat(target).take(*count))
                        })
                        .flatten()
                        .map(|target| self.valid_targets[*choosing][target])
                        .collect_vec();

                    let needs_targets = ability.needs_targets(db)[*choosing];

                    if targets.len() >= needs_targets || targets.len() >= self.valid_targets.len() {
                        *choosing += 1;

                        if *choosing == ability.effects(db).len() {
                            let targets = self
                                .choices
                                .iter()
                                .enumerate()
                                .map(|(idx, choices)| {
                                    choices
                                        .iter()
                                        .filter_map(|(target, count)| {
                                            target.map(|target| {
                                                std::iter::repeat(target).take(*count)
                                            })
                                        })
                                        .flatten()
                                        .map(|target| self.valid_targets[idx][target])
                                        .collect_vec()
                                })
                                .collect_vec();

                            vec![ActionResult::AddAbilityToStack {
                                source: self.source.unwrap(),
                                ability: *ability,
                                targets,
                            }]
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                }
            }
            UnresolvedActionResult::Attach(aura) => {
                let valid_targets = &self.valid_targets[0];

                if choice.is_none() && !self.optional && valid_targets.len() > 1 {
                    return vec![];
                } else if valid_targets.is_empty() {
                    return vec![ActionResult::PermanentToGraveyard(self.source.unwrap())];
                }

                vec![ActionResult::ApplyAuraToTarget {
                    aura: *aura,
                    target: choice.map_or(valid_targets[0], |choice| valid_targets[choice]),
                }]
            }
            UnresolvedActionResult::AddCardToStack { choosing } => {
                *self.choices[*choosing].entry(choice).or_default() += 1;
                let valid_targets = &self.valid_targets[*choosing];
                let choices = &self.choices[*choosing];

                let mut targets = choices
                    .iter()
                    .filter_map(|(target, count)| {
                        target.map(|target| std::iter::repeat(target).take(*count))
                    })
                    .flatten()
                    .map(|target| valid_targets[target])
                    .collect_vec();

                if targets.is_empty() && valid_targets.len() == 1 {
                    targets = valid_targets.clone();
                }

                let needs_targets = self.source.unwrap().needs_targets(db)[*choosing];
                if targets.len() >= needs_targets || targets.len() >= valid_targets.len() {
                    *choosing += 1;
                    if *choosing == self.source.unwrap().effects(db).len() {
                        let targets = self
                            .choices
                            .iter()
                            .enumerate()
                            .map(|(idx, choices)| {
                                choices
                                    .iter()
                                    .filter_map(|(target, count)| {
                                        target.map(|target| std::iter::repeat(target).take(*count))
                                    })
                                    .flatten()
                                    .map(|target| self.valid_targets[idx][target])
                                    .collect_vec()
                            })
                            .collect_vec();

                        vec![ActionResult::AddCardToStack {
                            card: self.source.unwrap(),
                            targets,
                        }]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            UnresolvedActionResult::OrganizeStack(stack_entries) => {
                *self.choices[0].entry(choice).or_default() += 1;
                if choice.is_none() {
                    debug!("Returning default entry order");
                    return vec![ActionResult::UpdateStackEntries(stack_entries.clone())];
                }

                let choices = &self.choices[0];
                let targets = choices
                    .iter()
                    .filter_map(|(target, count)| {
                        target.map(|target| std::iter::repeat(target).take(*count))
                    })
                    .flatten()
                    .collect_vec();

                if targets.len() < stack_entries.len() {
                    return vec![];
                }

                debug!("Target order {:?}", targets);

                let mut results = vec![];
                for choice in targets {
                    results.push(stack_entries[choice].clone());
                }

                vec![ActionResult::UpdateStackEntries(results)]
            }
            UnresolvedActionResult::SacrificePermanent => {
                *self.choices[0].entry(choice).or_default() += 1;
                if choice.is_none() {
                    return vec![];
                }

                let valid_targets = &self.valid_targets[0];
                let target = valid_targets[choice.unwrap()];

                let ActiveTarget::Battlefield { id } = target else {
                    unreachable!();
                };

                vec![ActionResult::PermanentToGraveyard(id)]
            }
        }
    }
}
