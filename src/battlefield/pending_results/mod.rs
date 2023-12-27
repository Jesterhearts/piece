pub mod choose_modes;
pub mod choose_scry;
pub mod choose_targets;
pub mod choosing_cast;
pub mod declaring_attackers;
pub mod library_or_graveyard;
pub mod organizing_stack;
pub mod pay_costs;

use std::{
    collections::{HashSet, VecDeque},
    fmt::Debug,
};

use enum_dispatch::enum_dispatch;
use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    abilities::GainMana,
    battlefield::{
        choose_scry::ChoosingScry,
        choosing_cast::ChoosingCast,
        organizing_stack::OrganizingStack,
        pay_costs::PayCost,
        pending_results::{
            choose_modes::ChooseModes, choose_targets::ChooseTargets,
            declaring_attackers::DeclaringAttackers, library_or_graveyard::LibraryOrGraveyard,
        },
        ActionResult, Battlefield,
    },
    effects::Effect,
    in_play::{AbilityId, AuraId, CardId, CastFrom, Database, OnBattlefield},
    player::{AllPlayers, Controller, Owner},
    stack::{ActiveTarget, StackEntry},
    turns::Turn,
};

#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum ResolutionResult {
    Complete,
    TryAgain,
    PendingChoice,
}

#[derive(Debug, Clone)]
pub enum Source {
    Card(CardId),
    Ability(AbilityId),
    Effect(Effect, CardId),
}

impl Source {
    fn mode_options(&self, db: &mut Database) -> Vec<(usize, String)> {
        match self {
            Source::Card(card) => card
                .modes(db)
                .unwrap()
                .0
                .into_iter()
                .map(|mode| {
                    mode.effects
                        .into_iter()
                        .map(|effect| effect.oracle_text)
                        .join(", ")
                })
                .enumerate()
                .collect_vec(),
            Source::Ability(ability) => {
                if let Some(gain) = ability.gain_mana_ability(db) {
                    match gain.gain {
                        GainMana::Specific { .. } => vec![],
                        GainMana::Choice { choices } => {
                            let mut result = vec![];
                            for (idx, choice) in choices.into_iter().enumerate() {
                                let mut add = "Add ".to_string();
                                for mana in choice {
                                    mana.push_mana_symbol(&mut add);
                                }
                                result.push((idx, add))
                            }
                            result
                        }
                    }
                } else {
                    vec![]
                }
            }
            Source::Effect(effect, _) => effect
                .modes()
                .into_iter()
                .map(|mode| {
                    mode.effects
                        .into_iter()
                        .map(|effect| effect.oracle_text)
                        .join(", ")
                })
                .enumerate()
                .collect_vec(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TargetSource {
    Effect(Effect),
    Aura(AuraId),
}

impl TargetSource {
    fn wants_targets(&self) -> usize {
        match self {
            TargetSource::Effect(effect) => effect.wants_targets(),
            TargetSource::Aura(_) => 1,
        }
    }

    fn needs_targets(&self) -> usize {
        match self {
            TargetSource::Effect(effect) => effect.needs_targets(),
            TargetSource::Aura(_) => 1,
        }
    }
}

#[derive(Debug)]
#[enum_dispatch(PendingResult)]
pub enum Pending {
    ChooseModes(ChooseModes),
    ChooseTargets(ChooseTargets),
    ChooseCast(ChoosingCast),
    DeclaringAttackers(DeclaringAttackers),
    LibraryOrGraveyard(LibraryOrGraveyard),
    ChoosingScry(ChoosingScry),
    OrganizingStack(OrganizingStack),
    PayCosts(PayCost),
}

#[enum_dispatch]
pub trait PendingResult: std::fmt::Debug {
    #[must_use]
    fn optional(&self, db: &Database, all_players: &AllPlayers) -> bool;

    #[must_use]
    fn recompute_targets(
        &mut self,
        _db: &mut Database,
        _already_chosen: &HashSet<ActiveTarget>,
    ) -> bool {
        false
    }

    #[must_use]
    fn options(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)>;

    #[must_use]
    fn description(&self, db: &Database) -> String;

    #[must_use]
    fn is_empty(&self) -> bool;

    #[must_use]
    fn make_choice(
        &mut self,
        db: &mut Database,
        all_players: &mut AllPlayers,
        choice: Option<usize>,
        results: &mut PendingResults,
    ) -> bool;
}

#[must_use]
#[derive(Debug, Default)]
pub struct PendingResults {
    pending: VecDeque<Pending>,

    chosen_modes: Vec<usize>,
    chosen_targets: Vec<Vec<ActiveTarget>>,
    all_chosen_targets: HashSet<ActiveTarget>,

    settled_effects: VecDeque<ActionResult>,

    apply_in_stages: bool,
    gain_mana: Option<AbilityId>,
    add_to_stack: Option<Source>,
    cast_from: Option<CastFrom>,

    x_is: Option<usize>,

    applied: bool,
}

impl PendingResults {
    pub fn add_gain_mana(&mut self, source: AbilityId) {
        self.gain_mana = Some(source);
        self.apply_in_stages = false;
    }

    pub fn add_ability_to_stack(&mut self, source: AbilityId) {
        self.add_to_stack = Some(Source::Ability(source));
        self.apply_in_stages = false;
    }

    pub fn add_card_to_stack(&mut self, source: CardId, from: CastFrom) {
        self.add_to_stack = Some(Source::Card(source));
        self.cast_from(from);
        self.apply_in_stages = false;
    }

    pub fn cast_from(&mut self, from: CastFrom) {
        self.cast_from = Some(from);
    }

    pub fn apply_in_stages(&mut self) {
        self.apply_in_stages = true;
        self.add_to_stack = None;
    }

    pub(crate) fn push_choose_scry(&mut self, controller: Controller, cards: Vec<CardId>) {
        self.pending
            .push_back(Pending::ChoosingScry(ChoosingScry::new(cards, controller)));
    }

    pub fn push_choose_cast(&mut self, card: CardId, paying_costs: bool, discovering: bool) {
        self.pending.push_back(Pending::ChooseCast(ChoosingCast {
            choosing_to_cast: vec![card],
            paying_costs,
            discovering,
        }));
    }

    pub fn chosen_modes(&self) -> &Vec<usize> {
        &self.chosen_modes
    }

    pub fn push_settled(&mut self, action: ActionResult) {
        self.settled_effects.push_back(action);
    }

    pub fn push_invalid_target(&mut self, target: ActiveTarget) {
        self.all_chosen_targets.insert(target);
    }

    pub fn all_currently_targeted(&self) -> &HashSet<ActiveTarget> {
        &self.all_chosen_targets
    }

    pub(crate) fn push_choose_library_or_graveyard(&mut self, card: CardId) {
        self.pending
            .push_back(Pending::LibraryOrGraveyard(LibraryOrGraveyard { card }));
    }

    pub fn push_choose_mode(&mut self, source: Source) {
        self.pending
            .push_back(Pending::ChooseModes(ChooseModes { source }));
    }

    pub fn push_chosen_mode(&mut self, choice: usize) {
        self.chosen_modes.push(choice);
    }

    pub fn push_choose_targets(&mut self, choice: ChooseTargets) {
        self.pending.push_back(Pending::ChooseTargets(choice));
    }

    pub fn push_pay_costs(&mut self, pay: PayCost) {
        if !pay.is_empty() {
            self.pending.push_back(Pending::PayCosts(pay));
        }
    }

    pub fn set_organize_stack(&mut self, db: &Database, mut entries: Vec<StackEntry>, turn: &Turn) {
        entries.sort_by_key(|e| {
            e.ty.source().controller(db) != Controller::from(turn.priority_player())
        });

        self.pending
            .push_back(Pending::OrganizingStack(OrganizingStack::new(entries)))
    }

    pub fn set_declare_attackers(
        &mut self,
        db: &mut Database,
        all_players: &AllPlayers,
        attacker: Owner,
    ) {
        let mut players = all_players.all_players();
        players.retain(|player| *player != attacker);
        debug!(
            "Attacking {:?}",
            players
                .iter()
                .map(|player| all_players[*player].name.clone())
                .collect_vec()
        );
        // TODO goad, etc.
        self.pending
            .push_back(Pending::DeclaringAttackers(DeclaringAttackers {
                candidates: attacker
                    .get_cards::<OnBattlefield>(db)
                    .into_iter()
                    .filter(|card| card.can_attack(db))
                    .collect_vec(),
                choices: IndexSet::default(),
                targets: vec![],
                valid_targets: players,
            }));
    }

    pub fn choices_optional(&self, db: &Database, all_players: &AllPlayers) -> bool {
        self.pending
            .front()
            .map(|pend| pend.optional(db, all_players))
            .unwrap_or(true)
    }

    pub fn options(&mut self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        for pending in self.pending.iter_mut() {
            let _ = pending.recompute_targets(db, &self.all_chosen_targets);
        }

        if let Some(pending) = self.pending.front_mut() {
            pending.options(db, all_players)
        } else {
            vec![]
        }
    }

    pub fn description(&self, db: &Database) -> String {
        if let Some(pending) = self.pending.front() {
            pending.description(db)
        } else {
            String::default()
        }
    }

    pub fn resolve(
        &mut self,
        db: &mut Database,
        all_players: &mut AllPlayers,
        turn: &Turn,
        choice: Option<usize>,
    ) -> ResolutionResult {
        assert!(!(self.add_to_stack.is_some() && self.apply_in_stages));
        debug!("Choosing {:?} for {:#?}", choice, self);

        let mut recomputed = false;
        for pend in self.pending.iter_mut() {
            recomputed |= pend.recompute_targets(db, &self.all_chosen_targets);
        }

        if recomputed {
            return ResolutionResult::TryAgain;
        }

        self.pending.retain(|pend| {
            let Pending::PayCosts(pay) = pend else {
                return true;
            };

            !pay.is_empty()
        });

        if self.pending.is_empty() {
            if let Some(source) = self.add_to_stack.take() {
                match source {
                    Source::Card(card) => {
                        self.settled_effects.push_back(ActionResult::CastCard {
                            card,
                            targets: self.chosen_targets.clone(),
                            from: self.cast_from.unwrap(),
                            x_is: self.x_is,
                            chosen_modes: self.chosen_modes.clone(),
                        });
                    }
                    Source::Ability(ability) => {
                        let source = ability.source(db);
                        self.settled_effects
                            .push_back(ActionResult::AddAbilityToStack {
                                source,
                                ability,
                                targets: self.chosen_targets.clone(),
                                x_is: self.x_is,
                            });
                    }
                    Source::Effect(_, _) => unreachable!(),
                }
            } else if let Some(id) = self.gain_mana.take() {
                let target = id.source(db).controller(db);
                let source = id.mana_source(db);
                let restriction = id.mana_restriction(db);
                if let Some(mana) = id.gain_mana_ability(db) {
                    match mana.gain {
                        GainMana::Specific { gains } => {
                            self.settled_effects.push_back(ActionResult::GainMana {
                                gain: gains,
                                target,
                                source,
                                restriction,
                            })
                        }
                        GainMana::Choice { choices } => {
                            let option = self.chosen_modes.pop().unwrap();
                            self.settled_effects.push_back(ActionResult::GainMana {
                                gain: choices[option].clone(),
                                target,
                                source,
                                restriction,
                            })
                        }
                    }
                }
            }

            if !self.settled_effects.is_empty() {
                self.applied = true;
                let results = Battlefield::apply_action_results(
                    db,
                    all_players,
                    turn,
                    self.settled_effects.make_contiguous(),
                );
                self.settled_effects.clear();
                self.extend(results);
            }

            if self.is_empty() {
                return ResolutionResult::Complete;
            }

            return ResolutionResult::TryAgain;
        }

        if self.apply_in_stages && !self.settled_effects.is_empty() {
            self.applied = true;
            let results = Battlefield::apply_action_results(
                db,
                all_players,
                turn,
                self.settled_effects.make_contiguous(),
            );
            self.settled_effects.clear();
            self.extend(results);

            for pend in self.pending.iter_mut() {
                if pend.recompute_targets(db, &self.all_chosen_targets) {
                    return ResolutionResult::TryAgain;
                }
            }
        }

        if let Some(mut next) = self.pending.pop_front() {
            if next.make_choice(db, all_players, choice, self) {
                ResolutionResult::TryAgain
            } else {
                self.pending.push_front(next);
                ResolutionResult::PendingChoice
            }
        } else {
            ResolutionResult::TryAgain
        }
    }

    pub fn extend(&mut self, results: PendingResults) {
        if results.is_empty() {
            return;
        }

        self.pending.extend(results.pending);
        self.settled_effects.extend(results.settled_effects);

        self.applied = results.applied;
        self.add_to_stack = results.add_to_stack;
        self.gain_mana = results.gain_mana;
        self.cast_from = results.cast_from;
        self.apply_in_stages = results.apply_in_stages;
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.settled_effects.is_empty()
    }

    pub fn only_immediate_results(&self, db: &Database, all_players: &AllPlayers) -> bool {
        self.pending.iter().all(|pend| match pend {
            Pending::ChooseTargets(targets) => targets.is_empty(),
            Pending::PayCosts(pay) => pay.autopay(db, all_players),
            _ => false,
        })
    }

    pub fn can_cancel(&self) -> bool {
        self.is_empty() || !self.applied
    }

    pub fn priority(&self, db: &Database, all_players: &AllPlayers, turn: &Turn) -> Owner {
        if let Some(pend) = self.pending.front() {
            match pend {
                Pending::DeclaringAttackers(declaring) => {
                    let mut all_players = all_players
                        .all_players()
                        .into_iter()
                        .collect::<HashSet<_>>();
                    for target in declaring.valid_targets.iter() {
                        all_players.remove(target);
                    }

                    all_players.into_iter().exactly_one().unwrap()
                }
                Pending::OrganizingStack(organizing) => {
                    if let Some(first) = organizing
                        .entries
                        .iter()
                        .enumerate()
                        .find(|(idx, _)| !organizing.choices.contains(idx))
                    {
                        first.1.ty.source().controller(db).into()
                    } else {
                        turn.priority_player()
                    }
                }
                _ => turn.priority_player(),
            }
        } else {
            turn.priority_player()
        }
    }
}