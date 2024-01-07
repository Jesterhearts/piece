pub(crate) mod choose_for_each_player;
pub(crate) mod choose_modes;
pub(crate) mod choose_targets;
pub(crate) mod choosing_cast;
pub(crate) mod declaring_attackers;
pub(crate) mod examine_top_cards;
pub(crate) mod library_or_graveyard;
pub(crate) mod organizing_stack;
pub(crate) mod pay_costs;

use std::{
    collections::{HashSet, VecDeque},
    fmt::Debug,
};

use enum_dispatch::enum_dispatch;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use tracing::Level;

use crate::{
    abilities::{Ability, GainMana, GainManaAbility},
    battlefield::{ActionResult, Battlefield},
    effects::{Destination, Effect, EffectBehaviors},
    in_play::{CardId, CastFrom, Database},
    pending_results::{
        choose_for_each_player::ChooseForEachPlayer, choose_modes::ChooseModes,
        choose_targets::ChooseTargets, choosing_cast::ChoosingCast,
        declaring_attackers::DeclaringAttackers, examine_top_cards::ExamineCards,
        library_or_graveyard::LibraryOrGraveyard, organizing_stack::OrganizingStack,
        pay_costs::PayCost,
    },
    player::{Controller, Owner},
    stack::{ActiveTarget, StackEntry},
};

#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum ResolutionResult {
    Complete,
    TryAgain,
    PendingChoice,
}

#[derive(Debug, Clone)]
pub(crate) enum Source {
    Card(CardId),
    Ability { source: CardId, ability: Ability },
    Effect(Effect, CardId),
}

impl Source {
    fn mode_options(&self, db: &mut Database) -> Vec<(usize, String)> {
        match self {
            Source::Card(card) => card
                .faceup_face(db)
                .modes
                .iter()
                .map(|mode| {
                    mode.effects
                        .iter()
                        .map(|effect| &effect.oracle_text)
                        .cloned()
                        .join(", ")
                })
                .enumerate()
                .collect_vec(),
            Source::Ability { ability, .. } => {
                if let Ability::Mana(gain) = ability {
                    match &gain.gain {
                        GainMana::Specific { .. } => vec![],
                        GainMana::Choice { choices } => {
                            let mut result = vec![];
                            for (idx, choice) in choices.iter().enumerate() {
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
pub(crate) enum TargetSource {
    Effect(Effect),
    Aura(CardId),
}

impl TargetSource {
    fn wants_targets(&self, db: &mut Database, source: CardId) -> usize {
        match self {
            TargetSource::Effect(effect) => effect.wants_targets(db, source),
            TargetSource::Aura(_) => 1,
        }
    }

    fn needs_targets(&self, db: &mut Database, source: CardId) -> usize {
        match self {
            TargetSource::Effect(effect) => effect.needs_targets(db, source),
            TargetSource::Aura(_) => 1,
        }
    }
}

#[derive(Debug)]
#[enum_dispatch(PendingResult)]
pub(crate) enum Pending {
    ChooseForEachPlayer(ChooseForEachPlayer),
    ChooseModes(ChooseModes),
    ChooseTargets(ChooseTargets),
    ChooseCast(ChoosingCast),
    DeclaringAttackers(DeclaringAttackers),
    LibraryOrGraveyard(LibraryOrGraveyard),
    ExamineCards(ExamineCards),
    OrganizingStack(OrganizingStack),
    PayCosts(PayCost),
}

#[enum_dispatch]
pub(crate) trait PendingResult: std::fmt::Debug {
    #[must_use]
    fn optional(&self, db: &Database) -> bool;

    #[must_use]
    fn cancelable(&self, db: &Database) -> bool {
        self.optional(db)
    }

    #[must_use]
    fn recompute_targets(
        &mut self,
        db: &mut Database,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> bool {
        let _ = db;
        let _ = already_chosen;
        false
    }

    #[must_use]
    fn options(&self, db: &mut Database) -> Vec<(usize, String)>;

    #[must_use]
    fn description(&self, db: &Database) -> String;

    #[must_use]
    fn is_empty(&self) -> bool;

    #[must_use]
    fn make_choice(
        &mut self,
        db: &mut Database,
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

    settled_effects: Vec<ActionResult>,

    apply_in_stages: bool,
    gain_mana: Option<(CardId, GainManaAbility)>,
    add_to_stack: Option<Source>,
    cast_from: Option<CastFrom>,

    x_is: Option<usize>,

    applied: bool,
}

impl PendingResults {
    pub(crate) fn add_gain_mana(&mut self, source: CardId, gain: GainManaAbility) {
        self.gain_mana = Some((source, gain));
    }

    pub(crate) fn add_ability_to_stack(&mut self, source: CardId, ability: Ability) {
        self.add_to_stack = Some(Source::Ability { source, ability });
    }

    pub(crate) fn add_card_to_stack(&mut self, source: CardId, from: CastFrom) {
        self.add_to_stack = Some(Source::Card(source));
        self.cast_from(from);
    }

    pub(crate) fn cast_from(&mut self, from: CastFrom) {
        self.cast_from = Some(from);
    }

    pub(crate) fn apply_in_stages(&mut self) {
        self.apply_in_stages = true;
        self.add_to_stack = None;
    }

    pub(crate) fn push_choose_scry(&mut self, cards: Vec<CardId>) {
        self.pending
            .push_back(Pending::ExamineCards(ExamineCards::new(
                examine_top_cards::Location::Library,
                cards,
                IndexMap::from([
                    (Destination::BottomOfLibrary, usize::MAX),
                    (Destination::TopOfLibrary, usize::MAX),
                ]),
            )));
    }

    pub(crate) fn push_choose_discard(&mut self, cards: Vec<CardId>, count: usize) {
        self.pending
            .push_back(Pending::ExamineCards(ExamineCards::new(
                examine_top_cards::Location::Hand,
                cards,
                IndexMap::from([(Destination::Graveyard, count)]),
            )));
    }

    pub(crate) fn push_examine_top_cards(&mut self, examining: ExamineCards) {
        self.pending.push_back(Pending::ExamineCards(examining));
    }

    pub(crate) fn push_choose_cast(&mut self, card: CardId, paying_costs: bool, discovering: bool) {
        self.pending.push_back(Pending::ChooseCast(ChoosingCast {
            choosing_to_cast: vec![card],
            paying_costs,
            discovering,
        }));
    }

    pub(crate) fn chosen_modes(&mut self) -> &mut Vec<usize> {
        &mut self.chosen_modes
    }

    pub(crate) fn push_settled(&mut self, action: ActionResult) {
        self.settled_effects.push(action);
    }

    pub(crate) fn push_invalid_target(&mut self, target: ActiveTarget) {
        self.all_chosen_targets.insert(target);
    }

    pub(crate) fn all_currently_targeted(&self) -> &HashSet<ActiveTarget> {
        &self.all_chosen_targets
    }

    pub(crate) fn push_choose_library_or_graveyard(&mut self, card: CardId) {
        self.pending
            .push_back(Pending::LibraryOrGraveyard(LibraryOrGraveyard { card }));
    }

    pub(crate) fn push_choose_mode(&mut self, source: Source) {
        self.pending
            .push_back(Pending::ChooseModes(ChooseModes { source }));
    }

    pub(crate) fn push_chosen_mode(&mut self, choice: usize) {
        self.chosen_modes.push(choice);
    }

    pub(crate) fn push_choose_for_each(&mut self, choice: ChooseForEachPlayer) {
        self.pending.push_back(Pending::ChooseForEachPlayer(choice));
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub(crate) fn push_choose_targets(&mut self, choice: ChooseTargets) {
        self.pending.push_back(Pending::ChooseTargets(choice));
    }

    pub(crate) fn push_pay_costs(&mut self, pay: PayCost) {
        if !pay.is_empty() {
            self.pending.push_back(Pending::PayCosts(pay));
        }
    }

    pub fn set_organize_stack(&mut self, db: &Database, mut entries: Vec<StackEntry>) {
        entries.sort_by_key(|e| {
            db[e.ty.source()].controller != Controller::from(db.turn.priority_player())
        });

        self.pending
            .push_back(Pending::OrganizingStack(OrganizingStack::new(entries)))
    }

    pub(crate) fn set_declare_attackers(&mut self, db: &mut Database, attacker: Owner) {
        let mut players = db.all_players.all_players();
        players.retain(|player| *player != attacker);
        debug!(
            "Attacking {:?}",
            players
                .iter()
                .map(|player| db.all_players[*player].name.clone())
                .collect_vec()
        );
        // TODO goad, etc.
        self.pending
            .push_back(Pending::DeclaringAttackers(DeclaringAttackers {
                candidates: db.battlefield[attacker]
                    .iter()
                    .filter(|card| card.can_attack(db))
                    .copied()
                    .collect_vec(),
                choices: IndexSet::default(),
                targets: vec![],
                valid_targets: players,
            }));
    }

    pub fn choices_optional(&self, db: &Database) -> bool {
        self.pending
            .front()
            .map(|pend| pend.optional(db))
            .unwrap_or(true)
    }

    pub fn options(&mut self, db: &mut Database) -> Vec<(usize, String)> {
        for pending in self.pending.iter_mut() {
            let _ = pending.recompute_targets(db, &self.all_chosen_targets);
        }

        if let Some(pending) = self.pending.front_mut() {
            pending.options(db)
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

    pub fn resolve(&mut self, db: &mut Database, choice: Option<usize>) -> ResolutionResult {
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
                        debug!("Casting card {}", db[card].modified_name);
                        self.settled_effects.push(ActionResult::CastCard {
                            card,
                            targets: self.chosen_targets.clone(),
                            from: self.cast_from.unwrap(),
                            x_is: self.x_is,
                            chosen_modes: self.chosen_modes.clone(),
                        });

                        self.chosen_modes.clear();
                    }
                    Source::Ability { source, ability } => {
                        self.settled_effects.push(ActionResult::AddAbilityToStack {
                            source,
                            ability,
                            targets: self.chosen_targets.clone(),
                            x_is: self.x_is,
                        });
                    }
                    Source::Effect(_, _) => unreachable!(),
                }
            } else if let Some((source, gain)) = self.gain_mana.take() {
                let target = db[source].controller;
                let source = gain.mana_source.unwrap_or_default();
                let restriction = gain.mana_restriction;
                match gain.gain {
                    GainMana::Specific { gains } => {
                        self.settled_effects.push(ActionResult::GainMana {
                            gain: gains,
                            target,
                            source,
                            restriction,
                        })
                    }
                    GainMana::Choice { choices } => {
                        let option = self.chosen_modes.pop().unwrap();
                        self.chosen_modes.clear();
                        self.settled_effects.push(ActionResult::GainMana {
                            gain: choices[option].clone(),
                            target,
                            source,
                            restriction,
                        })
                    }
                }
            }

            if !self.settled_effects.is_empty() {
                self.applied = true;
                let results = Battlefield::apply_action_results(db, &self.settled_effects);
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
            let results = Battlefield::apply_action_results(db, &self.settled_effects);
            self.settled_effects.clear();
            self.extend(results);

            for pend in self.pending.iter_mut() {
                if pend.recompute_targets(db, &self.all_chosen_targets) {
                    return ResolutionResult::TryAgain;
                }
            }
        }

        if let Some(mut next) = self.pending.pop_front() {
            if next.make_choice(db, choice, self) {
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

    pub fn only_immediate_results(&self, db: &Database) -> bool {
        self.pending.iter().all(|pend| match pend {
            Pending::ChooseTargets(targets) => targets.is_empty(),
            Pending::PayCosts(pay) => pay.autopay(db),
            _ => false,
        })
    }

    pub fn can_cancel(&self, db: &Database) -> bool {
        self.is_empty()
            || (self
                .pending
                .front()
                .map(|pend| pend.cancelable(db))
                .unwrap_or(true)
                && !self.applied)
    }

    pub fn priority(&self, db: &Database) -> Owner {
        if let Some(pend) = self.pending.front() {
            match pend {
                Pending::PayCosts(pay) => db[pay.source].controller.into(),
                Pending::DeclaringAttackers(declaring) => {
                    let mut all_players = db
                        .all_players
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
                        db[first.1.ty.source()].controller.into()
                    } else {
                        db.turn.priority_player()
                    }
                }
                _ => db.turn.priority_player(),
            }
        } else {
            db.turn.priority_player()
        }
    }
}
