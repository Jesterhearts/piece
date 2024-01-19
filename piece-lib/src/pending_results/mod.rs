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

use indexmap::IndexSet;
use itertools::Itertools;
use tracing::Level;

use crate::{
    abilities::Ability,
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::{CardId, CastFrom, Database, GainManaAbilityId},
    pending_results::{
        choose_for_each_player::ChooseForEachPlayer, choose_modes::ChooseModes,
        choose_targets::ChooseTargets, choosing_cast::ChoosingCast,
        declaring_attackers::DeclaringAttackers, examine_top_cards::ExamineCards,
        library_or_graveyard::LibraryOrGraveyard, organizing_stack::OrganizingStack,
        pay_costs::PayCost,
    },
    player::{Controller, Owner},
    protogen::{
        effects::{
            destination,
            effect::Effect,
            examine_top_cards::Dest,
            gain_mana::{Choice, Gain},
            Destination,
        },
        targets::Location,
    },
    stack::{ActiveTarget, StackEntry},
};

pub enum Options {
    MandatoryList(Vec<(usize, String)>),
    OptionalList(Vec<(usize, String)>),
    ListWithDefault(Vec<(usize, String)>),
}

impl Options {
    pub fn is_empty(&self) -> bool {
        match self {
            Options::MandatoryList(opts)
            | Options::OptionalList(opts)
            | Options::ListWithDefault(opts) => opts.is_empty(),
        }
    }
}

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

#[derive(Debug, Clone)]
pub(crate) enum CopySource {
    Card {
        card: CardId,
        controller: Controller,
        modes: Vec<usize>,
        x_is: Option<usize>,
    },
    Ability {
        source: CardId,
        ability: Ability,
        x_is: Option<usize>,
    },
}

impl Source {
    fn card(&self) -> CardId {
        match self {
            Source::Card(source) | Source::Ability { source, .. } | Source::Effect(_, source) => {
                *source
            }
        }
    }

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
                    match &db[*gain].ability.gain_mana.gain.as_ref().unwrap() {
                        Gain::Specific(_) => vec![],
                        Gain::Choice(Choice { choices, .. }) => {
                            let mut result = vec![];
                            for (idx, choice) in choices.iter().enumerate() {
                                let mut add = "Add ".to_string();
                                for mana in choice.gains.iter() {
                                    mana.enum_value().unwrap().push_mana_symbol(&mut add);
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
#[enum_delegate::implement(PendingResult)]
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

#[enum_delegate::register]
pub(crate) trait PendingResult {
    #[must_use]
    fn cancelable(&self, db: &Database) -> bool;

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
    fn options(&self, db: &mut Database) -> Options;

    #[must_use]
    fn target_for_option(&self, db: &Database, option: usize) -> Option<ActiveTarget>;

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
    gain_mana: Vec<(CardId, GainManaAbilityId)>,
    add_to_stack: VecDeque<Source>,
    copy_to_stack: Vec<CopySource>,
    cast_from: Option<CastFrom>,

    x_is: Option<usize>,

    applied: bool,
}

impl PendingResults {
    pub(crate) fn add_gain_mana(&mut self, source: CardId, gain: GainManaAbilityId) {
        self.gain_mana.push((source, gain));
    }

    pub(crate) fn add_ability_to_stack(&mut self, source: CardId, ability: Ability) {
        self.add_to_stack
            .push_back(Source::Ability { source, ability });
    }

    pub(crate) fn add_card_to_stack(&mut self, source: CardId, from: Option<CastFrom>) {
        self.add_to_stack.push_back(Source::Card(source));
        self.cast_from(from);
    }

    pub(crate) fn copy_ability_to_stack(
        &mut self,
        source: CardId,
        ability: Ability,
        _controller: Controller,
        x_is: Option<usize>,
    ) {
        self.copy_to_stack.push(CopySource::Ability {
            source,
            ability,
            x_is,
        });
    }

    pub(crate) fn copy_card_to_stack(
        &mut self,
        source: CardId,
        controller: Controller,
        modes: Vec<usize>,
        x_is: Option<usize>,
    ) {
        self.copy_to_stack.push(CopySource::Card {
            card: source,
            controller,
            modes,
            x_is,
        });
    }

    pub(crate) fn cast_from(&mut self, from: Option<CastFrom>) {
        self.cast_from = from;
    }

    pub(crate) fn apply_in_stages(&mut self) {
        self.apply_in_stages = true;
        self.add_to_stack.clear();
    }

    pub(crate) fn push_choose_scry(&mut self, cards: Vec<CardId>) {
        self.pending
            .push_back(Pending::ExamineCards(ExamineCards::new(
                Location::IN_LIBRARY,
                cards,
                vec![
                    Dest {
                        destination: protobuf::MessageField::some(Destination {
                            destination: Some(destination::Destination::BottomOfLibrary(
                                Default::default(),
                            )),
                            ..Default::default()
                        }),
                        count: u32::MAX,
                        ..Default::default()
                    },
                    Dest {
                        destination: protobuf::MessageField::some(Destination {
                            destination: Some(destination::Destination::TopOfLibrary(
                                Default::default(),
                            )),
                            ..Default::default()
                        }),
                        count: u32::MAX,
                        ..Default::default()
                    },
                ],
            )));
    }

    pub(crate) fn push_choose_discard(&mut self, cards: Vec<CardId>, count: u32) {
        self.pending
            .push_back(Pending::ExamineCards(ExamineCards::new(
                Location::IN_HAND,
                cards,
                vec![Dest {
                    destination: protobuf::MessageField::some(Destination {
                        destination: Some(destination::Destination::Graveyard(Default::default())),
                        ..Default::default()
                    }),
                    count,
                    ..Default::default()
                }],
            )));
    }

    pub(crate) fn push_examine_cards(&mut self, examining: ExamineCards) {
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

    pub fn options(&mut self, db: &mut Database) -> Options {
        for pending in self.pending.iter_mut() {
            let _ = pending.recompute_targets(db, &self.all_chosen_targets);
        }

        if let Some(pending) = self.pending.front_mut() {
            pending.options(db)
        } else {
            Options::OptionalList(vec![])
        }
    }

    pub fn target_for_option(&self, db: &Database, option: usize) -> Option<ActiveTarget> {
        if let Some(pending) = self.pending.front() {
            pending.target_for_option(db, option)
        } else {
            None
        }
    }

    pub fn description(&self, db: &Database) -> String {
        if let Some(pending) = self.pending.front() {
            pending.description(db)
        } else {
            String::default()
        }
    }

    #[instrument(level = Level::DEBUG, skip(db))]
    pub fn resolve(&mut self, db: &mut Database, choice: Option<usize>) -> ResolutionResult {
        event!(Level::DEBUG, "resolution");

        assert!(self.add_to_stack.is_empty() || !self.apply_in_stages);

        let mut recomputed = false;
        for pend in self.pending.iter_mut() {
            recomputed |= pend.recompute_targets(db, &self.all_chosen_targets);
        }

        if recomputed {
            return ResolutionResult::TryAgain;
        }

        if self.apply_in_stages && !self.settled_effects.is_empty() {
            self.applied = true;
            let results = ActionResult::apply_action_results(db, &self.settled_effects);
            self.settled_effects.clear();
            self.extend(results);

            if self.is_empty() {
                return ResolutionResult::Complete;
            }

            return ResolutionResult::TryAgain;
        }

        self.pending.retain(|pend| {
            let Pending::PayCosts(pay) = pend else {
                return true;
            };

            !pay.is_empty()
        });

        if self.pending.is_empty() {
            if let Some(source) = self.add_to_stack.pop_front() {
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

                self.chosen_modes.clear();
                self.chosen_targets.clear();
            } else if !self.gain_mana.is_empty() {
                for (source, gain) in self.gain_mana.drain(..) {
                    let target = db[source].controller;
                    let source = db[gain].ability.mana_source;
                    let restriction = db[gain].ability.mana_restriction;
                    match db[gain].ability.gain_mana.gain.as_ref().unwrap() {
                        Gain::Specific(specific) => {
                            self.settled_effects.push(ActionResult::GainMana {
                                gain: specific.gain.clone(),
                                target,
                                source,
                                restriction,
                            })
                        }
                        Gain::Choice(Choice { choices, .. }) => {
                            let option = self.chosen_modes.pop().unwrap();
                            self.chosen_modes.clear();
                            self.settled_effects.push(ActionResult::GainMana {
                                gain: choices[option].gains.clone(),
                                target,
                                source,
                                restriction,
                            })
                        }
                    }
                }
            } else if !self.copy_to_stack.is_empty() {
                for copy in self.copy_to_stack.drain(..) {
                    match copy {
                        CopySource::Card {
                            card,
                            controller,
                            modes,
                            x_is,
                        } => {
                            self.settled_effects.push(ActionResult::CopyCardInStack {
                                card,
                                controller,
                                targets: self.chosen_targets.clone(),
                                x_is,
                                chosen_modes: modes,
                            });

                            self.chosen_modes.clear();
                        }
                        CopySource::Ability {
                            source,
                            ability,
                            x_is,
                        } => self.settled_effects.push(ActionResult::CopyAbility {
                            source,
                            ability,
                            targets: self.chosen_targets.clone(),
                            x_is,
                        }),
                    }
                }

                self.chosen_targets.clear();
                self.chosen_modes.clear();
            }

            if !self.settled_effects.is_empty() {
                self.applied = true;
                let results = ActionResult::apply_action_results(db, &self.settled_effects);
                self.settled_effects.clear();
                self.extend(results);
            }

            if self.is_empty() {
                return ResolutionResult::Complete;
            }

            return ResolutionResult::TryAgain;
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
        self.gain_mana.extend(results.gain_mana);

        self.applied = results.applied;
        self.add_to_stack = results.add_to_stack;
        self.cast_from = results.cast_from;
        self.x_is = results.x_is;
        self.apply_in_stages = results.apply_in_stages;
    }

    pub fn is_empty(&self) -> bool {
        self.add_to_stack.is_empty()
            && self.pending.is_empty()
            && self.settled_effects.is_empty()
            && self.gain_mana.is_empty()
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
