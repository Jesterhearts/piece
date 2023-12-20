use std::collections::{HashSet, VecDeque};

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    abilities::GainMana,
    battlefield::{ActionResult, Battlefield},
    controller::ControllerRestriction,
    effects::{DealDamage, Destination, Effect, Mill, TutorLibrary},
    in_play::{AbilityId, AuraId, CardId, CastFrom, Database, ModifierId, OnBattlefield},
    mana::{Mana, ManaCost},
    player::{AllPlayers, Controller, Owner},
    stack::{ActiveTarget, Stack, StackEntry},
    targets::Restriction,
};

#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum ResolutionResult {
    Complete,
    TryAgain,
    PendingChoice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Card(CardId),
    Ability(AbilityId),
}

impl Source {
    fn card(&self, db: &Database) -> CardId {
        match self {
            Source::Card(id) => *id,
            Source::Ability(id) => id.source(db),
        }
    }

    fn mode_options(&self, db: &mut Database) -> Vec<(usize, String)> {
        match self {
            Source::Card(_) => todo!(),
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectOrAura {
    Effect(Effect),
    Aura(AuraId),
}

impl EffectOrAura {
    fn wants_targets(&self) -> usize {
        match self {
            EffectOrAura::Effect(effect) => effect.wants_targets(),
            EffectOrAura::Aura(_) => 1,
        }
    }

    fn needs_targets(&self) -> usize {
        match self {
            EffectOrAura::Effect(effect) => effect.needs_targets(),
            EffectOrAura::Aura(_) => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooseTargets {
    effect_or_aura: EffectOrAura,
    valid_targets: Vec<ActiveTarget>,
    chosen: IndexMap<usize, usize>,
    skipping_remainder: bool,
}

impl ChooseTargets {
    pub fn new(effect_or_aura: EffectOrAura, valid_targets: Vec<ActiveTarget>) -> Self {
        Self {
            effect_or_aura,
            valid_targets,
            chosen: Default::default(),
            skipping_remainder: false,
        }
    }

    pub fn recompute_targets(&mut self, db: &mut Database, source: Source) {
        let card = source.card(db);
        let controller = card.controller(db);
        match &self.effect_or_aura {
            EffectOrAura::Effect(effect) => {
                self.valid_targets = card.targets_for_effect(db, controller, effect);
            }
            EffectOrAura::Aura(_) => {
                self.valid_targets = card.targets_for_aura(db).unwrap();
            }
        }
    }

    #[must_use]
    pub fn choose_targets(&mut self, choice: Option<usize>) -> bool {
        debug!("choosing target: {:?}", choice);
        if let Some(choice) = choice {
            if self.valid_targets.is_empty() {
                true
            } else if choice >= self.valid_targets.len() {
                false
            } else {
                *self.chosen.entry(choice).or_default() += 1;
                true
            }
        } else if self.valid_targets.len() == 1 {
            debug!("Choosing default only target");
            *self.chosen.entry(0).or_default() += 1;
            true
        } else if self.can_skip() {
            self.skipping_remainder = true;
            true
        } else {
            false
        }
    }

    pub fn into_effect(self) -> EffectOrAura {
        self.effect_or_aura
    }

    pub fn into_chosen_targets_and_effect(self) -> (Vec<ActiveTarget>, EffectOrAura) {
        let mut results = vec![];
        for choice in self
            .chosen
            .into_iter()
            .flat_map(|(choice, count)| std::iter::repeat(choice).take(count))
        {
            results.push(self.valid_targets[choice]);
        }

        (results, self.effect_or_aura)
    }

    pub fn into_chosen_targets(self) -> Vec<ActiveTarget> {
        self.into_chosen_targets_and_effect().0
    }

    pub fn chosen_targets_count(&self) -> usize {
        self.chosen.values().sum()
    }

    pub fn choices_complete(&self) -> bool {
        self.chosen_targets_count() >= self.effect_or_aura.wants_targets()
            || self.chosen_targets_count() >= self.valid_targets.len()
            || (self.can_skip() && self.skipping_remainder)
    }

    pub fn can_skip(&self) -> bool {
        self.chosen_targets_count() >= self.effect_or_aura.needs_targets()
            || self.chosen_targets_count() >= self.valid_targets.len()
    }

    fn options(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        self.valid_targets
            .iter()
            .enumerate()
            .map(|(idx, target)| (idx, target.display(db, all_players)))
            .collect_vec()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SacrificePermanent {
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: Option<CardId>,
}

impl SacrificePermanent {
    pub fn new(restrictions: Vec<Restriction>) -> Self {
        Self {
            restrictions,
            valid_targets: Default::default(),
            chosen: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendMana {
    paying: IndexMap<ManaCost, usize>,
    paid: IndexMap<ManaCost, IndexMap<Mana, usize>>,
}

impl SpendMana {
    pub fn new(mut mana: Vec<ManaCost>) -> Self {
        mana.sort();

        let mut paying = IndexMap::default();
        for cost in mana {
            *paying.entry(cost).or_default() += 1;
        }
        let mut paid = IndexMap::default();
        paid.entry(ManaCost::X).or_default();

        Self { paying, paid }
    }

    pub fn first_unpaid_x_always_unpaid(&self) -> Option<ManaCost> {
        let unpaid = self
            .paying
            .iter()
            .find(|(paying, required)| {
                let required = match paying {
                    ManaCost::Generic(count) => *count,
                    ManaCost::X => usize::MAX,
                    _ => **required,
                };

                self.paid
                    .get(*paying)
                    .map(|paid| {
                        let paid = paid.values().sum::<usize>();
                        paid < required
                    })
                    .unwrap_or(true)
            })
            .map(|(paying, _)| *paying);
        debug!("First unpaid: {:?}", unpaid);
        unpaid
    }

    pub fn first_unpaid(&self) -> Option<ManaCost> {
        self.first_unpaid_x_always_unpaid()
            .filter(|unpaid| !matches!(unpaid, ManaCost::X))
    }

    pub fn paid(&self) -> bool {
        self.first_unpaid().is_none()
    }

    pub fn paying(&self) -> Vec<Mana> {
        self.paid
            .values()
            .flat_map(|paid| {
                paid.iter()
                    .flat_map(|(mana, count)| std::iter::repeat(*mana).take(*count))
            })
            .collect_vec()
    }

    fn description(&self) -> String {
        match self.first_unpaid() {
            Some(ManaCost::Generic(_)) => "generic mana".to_string(),
            Some(ManaCost::X) => "X".to_string(),
            _ => String::default(),
        }
    }

    fn x_is(&self) -> Option<usize> {
        self.paid
            .get(&ManaCost::X)
            .map(|paid| paid.values().sum::<usize>())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayCost {
    SacrificePermanent(SacrificePermanent),
    SpendMana(SpendMana),
}

impl PayCost {
    fn choice_optional(&self, all_players: &AllPlayers, player: Owner) -> bool {
        match self {
            PayCost::SacrificePermanent(_) => false,
            PayCost::SpendMana(spend) => {
                let pool_post_pay = all_players[player].pool_post_pay(&spend.paying()).unwrap();
                let first_unpaid = spend.first_unpaid().unwrap();
                match first_unpaid {
                    ManaCost::Generic(count) => {
                        if count == 1 {
                            pool_post_pay.max().is_some()
                        } else {
                            false
                        }
                    }
                    ManaCost::X => false,
                    unpaid => pool_post_pay.can_spend(unpaid),
                }
            }
        }
    }

    fn paid(&self) -> bool {
        match self {
            PayCost::SacrificePermanent(sac) => sac.chosen.is_some(),
            PayCost::SpendMana(spend) => spend.paid(),
        }
    }

    fn options(
        &self,
        db: &Database,
        all_players: &AllPlayers,
        player: Owner,
        all_targets: &HashSet<ActiveTarget>,
    ) -> Vec<(usize, String)> {
        match self {
            PayCost::SacrificePermanent(sac) => sac
                .valid_targets
                .iter()
                .enumerate()
                .filter(|(_, target)| {
                    !all_targets.contains(&ActiveTarget::Battlefield { id: **target })
                })
                .map(|(idx, target)| (idx, format!("{} - ({})", target.name(db), target.id(db),)))
                .collect_vec(),
            PayCost::SpendMana(spend) => {
                let pool_post_paid = all_players[player].pool_post_pay(&spend.paying());
                if pool_post_paid.is_none() || pool_post_paid.unwrap().max().is_none() {
                    return vec![];
                }
                let pool_post_paid = pool_post_paid.unwrap();

                match spend.first_unpaid() {
                    Some(ManaCost::Generic(_) | ManaCost::X) => pool_post_paid
                        .available_mana()
                        .map(|(count, mana)| {
                            let mut result = format!("({}) ", count);
                            mana.push_mana_symbol(&mut result);
                            result
                        })
                        .enumerate()
                        .collect_vec(),
                    Some(ManaCost::White) => {
                        let mut result = format!("({}) ", pool_post_paid.white_mana);
                        Mana::White.push_mana_symbol(&mut result);
                        vec![(0, result)]
                    }
                    Some(ManaCost::Blue) => {
                        let mut result = format!("({}) ", pool_post_paid.blue_mana);
                        Mana::Blue.push_mana_symbol(&mut result);
                        vec![(1, result)]
                    }
                    Some(ManaCost::Black) => {
                        let mut result = format!("({}) ", pool_post_paid.black_mana);
                        Mana::Black.push_mana_symbol(&mut result);
                        vec![(2, result)]
                    }
                    Some(ManaCost::Red) => {
                        let mut result = format!("({}) ", pool_post_paid.red_mana);
                        Mana::Red.push_mana_symbol(&mut result);
                        vec![(3, result)]
                    }
                    Some(ManaCost::Green) => {
                        let mut result = format!("({}) ", pool_post_paid.green_mana);
                        Mana::Green.push_mana_symbol(&mut result);
                        vec![(4, result)]
                    }
                    Some(ManaCost::Colorless) => {
                        let mut result = format!("({}) ", pool_post_paid.colorless_mana);
                        Mana::Colorless.push_mana_symbol(&mut result);
                        vec![(5, result)]
                    }
                    None => vec![],
                }
            }
        }
    }

    fn compute_targets(
        &mut self,
        db: &mut Database,
        source: Source,
        already_chosen: &HashSet<ActiveTarget>,
    ) {
        if let PayCost::SacrificePermanent(sac) = self {
            let card = source.card(db);
            let controller = card.controller(db);
            let valid_targets = controller
                .get_cards::<OnBattlefield>(db)
                .into_iter()
                .filter(|target| {
                    !already_chosen.contains(&ActiveTarget::Battlefield { id: *target })
                        && target.passes_restrictions(
                            db,
                            card,
                            ControllerRestriction::You,
                            &sac.restrictions,
                        )
                })
                .collect_vec();
            sac.valid_targets = valid_targets;
        }
    }

    fn choose_pay(
        &mut self,
        all_players: &mut AllPlayers,
        player: Owner,
        all_targets: &HashSet<ActiveTarget>,
        choice: Option<usize>,
    ) -> bool {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent {
                valid_targets,
                chosen,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice];
                    if !all_targets.contains(&ActiveTarget::Battlefield { id: target }) {
                        *chosen = Some(target);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            PayCost::SpendMana(spend) => {
                if choice.is_none() {
                    let mut pool_post_pay =
                        all_players[player].pool_post_pay(&spend.paying()).unwrap();
                    let Some(first_unpaid) = spend.first_unpaid() else {
                        return self.paid();
                    };

                    if pool_post_pay.can_spend(first_unpaid) {
                        let mana = match first_unpaid {
                            ManaCost::White => Mana::White,
                            ManaCost::Blue => Mana::Blue,
                            ManaCost::Black => Mana::Black,
                            ManaCost::Red => Mana::Red,
                            ManaCost::Green => Mana::Green,
                            ManaCost::Colorless => Mana::Colorless,
                            ManaCost::Generic(count) => {
                                for _ in 0..count {
                                    let max = pool_post_pay.max().unwrap();
                                    assert!(pool_post_pay.spend(max));
                                    *spend
                                        .paid
                                        .entry(first_unpaid)
                                        .or_default()
                                        .entry(max)
                                        .or_default() += 1;
                                }

                                return !matches!(
                                    spend.first_unpaid_x_always_unpaid(),
                                    Some(ManaCost::X)
                                );
                            }
                            ManaCost::X => {
                                return true;
                            }
                        };
                        *spend
                            .paid
                            .entry(first_unpaid)
                            .or_default()
                            .entry(mana)
                            .or_default() += 1;
                    }

                    return self.paid();
                }

                let mana = Mana::iter().nth(choice.unwrap()).unwrap();
                let cost = spend.first_unpaid_x_always_unpaid().unwrap();
                *spend.paid.entry(cost).or_default().entry(mana).or_default() += 1;

                if all_players[player].can_spend_mana(&spend.paying()) {
                    !matches!(spend.first_unpaid_x_always_unpaid(), Some(ManaCost::X))
                } else {
                    false
                }
            }
        }
    }

    fn results(&self, player: Controller) -> ActionResult {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent { chosen, .. }) => {
                ActionResult::PermanentToGraveyard(chosen.unwrap())
            }
            PayCost::SpendMana(spend) => ActionResult::SpendMana(player, spend.paying()),
        }
    }

    fn description(&self) -> String {
        match self {
            PayCost::SacrificePermanent(_) => "sacrificing a permanent".to_string(),
            PayCost::SpendMana(spend) => spend.description(),
        }
    }

    fn x_is(&self) -> Option<usize> {
        match self {
            PayCost::SacrificePermanent(_) => None,
            PayCost::SpendMana(spend) => spend.x_is(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizingStack {
    entries: Vec<StackEntry>,
    choices: IndexSet<usize>,
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PendingResults {
    source: Option<Source>,

    choose_modes: VecDeque<()>,
    choose_targets: VecDeque<ChooseTargets>,
    pay_costs: VecDeque<PayCost>,
    choosing_to_cast: Vec<CardId>,

    organizing_stack: Option<OrganizingStack>,

    chosen_modes: Vec<usize>,
    chosen_targets: Vec<Vec<ActiveTarget>>,
    all_chosen_targets: HashSet<ActiveTarget>,

    settled_effects: VecDeque<ActionResult>,

    apply_in_stages: bool,
    add_to_stack: bool,
    cast_from: Option<CastFrom>,
    paying_costs: bool,

    x_is: Option<usize>,

    applied: bool,
}

impl<const T: usize> From<[ActionResult; T]> for PendingResults {
    fn from(value: [ActionResult; T]) -> Self {
        Self {
            settled_effects: VecDeque::from(value),
            ..Default::default()
        }
    }
}

impl<const T: usize> From<(CardId, bool, [ActionResult; T])> for PendingResults {
    fn from((source, apply_in_stages, value): (CardId, bool, [ActionResult; T])) -> Self {
        Self {
            source: Some(Source::Card(source)),
            settled_effects: VecDeque::from(value),
            apply_in_stages,
            applied: apply_in_stages,
            ..Default::default()
        }
    }
}

impl PendingResults {
    pub fn new(source: Source) -> Self {
        Self {
            source: Some(source),
            ..Default::default()
        }
    }

    pub fn add_ability_to_stack(&mut self) {
        self.add_to_stack = true;
        self.apply_in_stages = false;
    }

    pub fn add_card_to_stack(&mut self, from: CastFrom) {
        self.add_to_stack = true;
        self.cast_from(from)
    }

    pub fn cast_from(&mut self, from: CastFrom) {
        self.cast_from = Some(from);
    }

    pub fn apply_in_stages(&mut self) {
        self.applied = true;
        self.apply_in_stages = true;
        self.add_to_stack = false;
    }

    pub fn push_choose_cast(&mut self, card: CardId, paying_costs: bool) {
        self.choosing_to_cast.push(card);
        self.paying_costs = paying_costs;
    }

    pub fn push_settled(&mut self, action: ActionResult) {
        self.settled_effects.push_back(action);
    }

    pub fn push_choose_mode(&mut self) {
        self.choose_modes.push_back(());
    }

    pub fn push_choose_targets(&mut self, choice: ChooseTargets) {
        self.choose_targets.push_back(choice);
    }

    pub fn push_pay_costs(&mut self, pay: PayCost) {
        self.pay_costs.push_back(pay);
    }

    pub fn set_organize_stack(&mut self, entries: Vec<StackEntry>) {
        self.organizing_stack = Some(OrganizingStack {
            entries,
            choices: Default::default(),
        });
    }

    pub fn choices_optional(&self, db: &Database, all_players: &AllPlayers) -> bool {
        if self.choose_modes.front().is_some() {
            false
        } else if let Some(choosing) = self.choose_targets.front() {
            choosing.valid_targets.len() <= 1
        } else if self.organizing_stack.is_some() {
            true
        } else if !self.pay_costs.is_empty() {
            self.pay_costs.iter().all(|cost| {
                cost.choice_optional(all_players, self.source.unwrap().card(db).owner(db))
            })
        } else {
            true
        }
    }

    pub fn options(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        if self.choose_modes.front().is_some() {
            self.source.unwrap().mode_options(db)
        } else if let Some(choosing) = self.choose_targets.front() {
            choosing.options(db, all_players)
        } else if let Some(choosing) = self.pay_costs.front() {
            choosing.options(
                db,
                all_players,
                self.source.unwrap().card(db).owner(db),
                &self.all_chosen_targets,
            )
        } else if !self.choosing_to_cast.is_empty() {
            self.choosing_to_cast
                .iter()
                .enumerate()
                .map(|(idx, card)| (idx, card.name(db)))
                .collect_vec()
        } else if let Some(stack_org) = self.organizing_stack.as_ref() {
            stack_org
                .entries
                .iter()
                .enumerate()
                .filter(|(idx, _)| !stack_org.choices.contains(idx))
                .map(|(idx, entry)| (idx, entry.display(db)))
                .collect_vec()
        } else {
            vec![]
        }
    }

    pub fn description(&self, _db: &Database) -> String {
        if self.choose_modes.front().is_some() {
            "mode".to_string()
        } else if self.choose_targets.front().is_some() {
            "targets".to_string()
        } else if let Some(pay) = self.pay_costs.front() {
            pay.description()
        } else if !self.choosing_to_cast.is_empty() {
            "spells to cast".to_string()
        } else if self.organizing_stack.is_some() {
            "stack order".to_string()
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
        assert!(!(self.add_to_stack && self.apply_in_stages));
        debug!("Choosing {:?} for {:#?}", choice, self);

        if self.choose_modes.is_empty()
            && self.choose_targets.is_empty()
            && self.pay_costs.is_empty()
            && self.choosing_to_cast.is_empty()
            && self.organizing_stack.is_none()
        {
            if self.add_to_stack {
                match self.source.unwrap() {
                    Source::Card(card) => {
                        self.settled_effects.push_back(ActionResult::CastCard {
                            card,
                            targets: self.chosen_targets.clone(),
                            from: self.cast_from.unwrap(),
                            x_is: self.x_is,
                        });
                    }
                    Source::Ability(ability) => {
                        let source = ability.source(db);
                        self.settled_effects
                            .push_back(ActionResult::AddAbilityToStack {
                                source,
                                ability,
                                targets: self.chosen_targets.clone(),
                            });
                    }
                }
                self.add_to_stack = false;
            } else if let Some(Source::Ability(id)) = self.source {
                let controller = self.source.unwrap().card(db).controller(db);
                if let Some(mana) = id.gain_mana_ability(db) {
                    match mana.gain {
                        GainMana::Specific { gains } => {
                            self.settled_effects.push_back(ActionResult::GainMana {
                                gain: gains,
                                target: controller,
                            })
                        }
                        GainMana::Choice { choices } => {
                            let option = self.chosen_modes.pop().unwrap();
                            self.settled_effects.push_back(ActionResult::GainMana {
                                gain: choices[option].clone(),
                                target: controller,
                            })
                        }
                    }
                }
                self.source = None;
            }

            self.applied = true;
            let results = Battlefield::apply_action_results(
                db,
                all_players,
                self.settled_effects.make_contiguous(),
            );
            self.settled_effects.clear();
            self.extend(results);
            if self.is_empty() {
                return ResolutionResult::Complete;
            }

            return ResolutionResult::TryAgain;
        }

        if self.apply_in_stages {
            self.applied = true;
            let results = Battlefield::apply_action_results(
                db,
                all_players,
                self.settled_effects.make_contiguous(),
            );
            self.settled_effects.clear();
            self.extend(results);

            for choice in self.choose_targets.iter_mut() {
                choice.recompute_targets(db, self.source.unwrap());
            }
        }

        if let Some(choosing) = self.choose_modes.pop_front() {
            if let Some(choice) = choice {
                self.chosen_modes.push(choice);
            } else {
                self.choose_modes.push_front(choosing);
            }

            if self.choose_modes.is_empty() {
                ResolutionResult::TryAgain
            } else {
                ResolutionResult::PendingChoice
            }
        } else if let Some(mut choosing) = self.choose_targets.pop_front() {
            if choosing.choose_targets(choice) {
                if choosing.choices_complete() {
                    let (choices, effect_or_aura) = choosing.into_chosen_targets_and_effect();

                    if !self.add_to_stack {
                        let player = self.source.unwrap().card(db).controller(db);
                        match effect_or_aura {
                            EffectOrAura::Effect(effect) => {
                                self.push_effect_results(db, player, effect, choices.clone());
                            }
                            EffectOrAura::Aura(aura) => {
                                self.settled_effects
                                    .push_back(ActionResult::ApplyAuraToTarget {
                                        aura,
                                        target: self.chosen_targets.pop().unwrap().pop().unwrap(),
                                    })
                            }
                        }
                    } else {
                        self.all_chosen_targets.extend(choices.iter().copied());
                        self.chosen_targets.push(choices.clone());
                    }

                    if !self.source.unwrap().card(db).target_individually(db) {
                        let player = self.source.unwrap().card(db).controller(db);
                        for choosing in self.choose_targets.drain(..).collect_vec() {
                            let effect_or_aura = choosing.into_effect();

                            if self.add_to_stack {
                                self.chosen_targets.push(choices.clone());
                            } else {
                                match effect_or_aura {
                                    EffectOrAura::Effect(effect) => {
                                        self.push_effect_results(
                                            db,
                                            player,
                                            effect,
                                            choices.clone(),
                                        );
                                    }
                                    EffectOrAura::Aura(aura) => self.settled_effects.push_back(
                                        ActionResult::ApplyAuraToTarget {
                                            aura,
                                            target: self
                                                .chosen_targets
                                                .pop()
                                                .unwrap()
                                                .pop()
                                                .unwrap(),
                                        },
                                    ),
                                }
                            }
                        }
                    }

                    if self.choose_targets.is_empty() {
                        for cost in self.pay_costs.iter_mut() {
                            cost.compute_targets(
                                db,
                                self.source.unwrap(),
                                &self.all_chosen_targets,
                            );
                        }
                    }
                } else {
                    self.choose_targets.push_front(choosing);
                }
                ResolutionResult::TryAgain
            } else {
                self.choose_targets.push_front(choosing);
                ResolutionResult::PendingChoice
            }
        } else if let Some(mut pay) = self.pay_costs.pop_front() {
            debug!("Paying costs");
            let player = self.source.unwrap().card(db).controller(db);
            if pay.choose_pay(all_players, player.into(), &self.all_chosen_targets, choice) {
                if pay.paid() {
                    self.x_is = pay.x_is();
                    self.settled_effects.push_back(pay.results(player));
                } else {
                    self.pay_costs.push_front(pay);
                }
                ResolutionResult::TryAgain
            } else {
                self.pay_costs.push_front(pay);
                ResolutionResult::PendingChoice
            }
        } else if !self.choosing_to_cast.is_empty() {
            if let Some(choice) = choice {
                let results = Stack::move_card_to_stack_from_exile(
                    db,
                    self.choosing_to_cast.remove(choice),
                    self.paying_costs,
                );
                self.extend(results);
                ResolutionResult::TryAgain
            } else {
                self.choosing_to_cast.clear();
                ResolutionResult::TryAgain
            }
        } else if let Some(organizing) = self.organizing_stack.as_mut() {
            if let Some(choice) = choice {
                organizing.choices.insert(choice);

                debug!("Chosen {:?}", organizing.choices);

                if organizing.choices.len() == organizing.entries.len() {
                    let entries = organizing
                        .choices
                        .iter()
                        .map(|choice| organizing.entries[*choice].clone())
                        .collect_vec();

                    self.settled_effects
                        .push_back(ActionResult::UpdateStackEntries(entries));
                    self.organizing_stack = None;
                    ResolutionResult::TryAgain
                } else {
                    ResolutionResult::PendingChoice
                }
            } else {
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

        self.source = results.source;
        self.choose_modes.extend(results.choose_modes);
        self.choose_targets.extend(results.choose_targets);
        self.pay_costs.extend(results.pay_costs);
        self.choosing_to_cast.extend(results.choosing_to_cast);
        self.settled_effects.extend(results.settled_effects);

        self.organizing_stack = results.organizing_stack;
        self.cast_from = results.cast_from;
        self.apply_in_stages = results.apply_in_stages;
        self.add_to_stack = results.add_to_stack;
        self.paying_costs = results.paying_costs;
    }

    pub fn is_empty(&self) -> bool {
        self.choose_modes.is_empty()
            && self.choose_targets.is_empty()
            && self.pay_costs.is_empty()
            && self.choosing_to_cast.is_empty()
            && self.organizing_stack.is_none()
            && self.settled_effects.is_empty()
    }

    pub fn only_immediate_results(&self, db: &Database, all_players: &AllPlayers) -> bool {
        (self.choosing_to_cast.is_empty() && self.choose_modes.is_empty())
            && ((self.choose_targets.is_empty()
                && self.pay_costs.is_empty()
                && self.organizing_stack.is_none())
                || (self
                    .choose_targets
                    .iter()
                    .all(|choose| choose.valid_targets.is_empty())
                    && self.pay_costs.iter().all(|pay| {
                        pay.choice_optional(all_players, self.source.unwrap().card(db).owner(db))
                    })))
    }

    fn push_effect_results(
        &mut self,
        db: &mut Database,
        player: Controller,
        effect: Effect,
        mut targets: Vec<ActiveTarget>,
    ) {
        match effect {
            Effect::CopyOfAnyCreatureNonTargeting => {
                self.settled_effects
                    .push_front(ActionResult::CloneCreatureNonTargeting {
                        source: self.source.unwrap().card(db),
                        target: targets.first().copied(),
                    })
            }
            Effect::CreateTokenCopy { modifiers } => {
                let Some(ActiveTarget::Battlefield { id }) = targets.pop() else {
                    unreachable!()
                };
                self.settled_effects
                    .push_front(ActionResult::CreateTokenCopyOf {
                        target: id,
                        modifiers: modifiers.clone(),
                        controller: player,
                    })
            }
            Effect::DealDamage(DealDamage { quantity, .. }) => {
                self.settled_effects.push_front(ActionResult::DamageTarget {
                    quantity,
                    target: targets.pop().unwrap(),
                });
            }
            Effect::ExileTargetCreature => {
                self.settled_effects
                    .push_front(ActionResult::ExileTarget(targets.pop().unwrap()));
            }
            Effect::ExileTargetCreatureManifestTopOfLibrary => {
                let target = targets.pop().unwrap();
                let ActiveTarget::Battlefield { id } = target else {
                    unreachable!()
                };

                self.settled_effects
                    .push_front(ActionResult::ExileTarget(target));
                self.settled_effects
                    .push_front(ActionResult::ManifestTopOfLibrary(id.controller(db)))
            }
            Effect::GainCounter(counter) => {
                let target = targets.pop().unwrap();
                let ActiveTarget::Battlefield { id } = target else {
                    unreachable!()
                };
                self.settled_effects.push_front(ActionResult::AddCounters {
                    source: id,
                    target: id,
                    counter,
                });
            }
            Effect::Mill(Mill { count, .. }) => self
                .settled_effects
                .push_front(ActionResult::Mill { count, targets }),
            Effect::ModifyTarget(modifier) => {
                let card = self.source.unwrap().card(db);
                let modifier = ModifierId::upload_temporary_modifier(db, card, &modifier);
                self.settled_effects
                    .push_front(ActionResult::ModifyCreatures { targets, modifier });
            }
            Effect::ReturnFromGraveyardToBattlefield(_) => {
                self.settled_effects
                    .push_front(ActionResult::ReturnFromGraveyardToBattlefield { targets });
            }
            Effect::ReturnFromGraveyardToLibrary(_) => {
                self.settled_effects
                    .push_front(ActionResult::ReturnFromGraveyardToLibrary { targets });
            }
            Effect::TargetToTopOfLibrary { .. } => {
                self.settled_effects
                    .push_front(ActionResult::ReturnFromBattlefieldToLibrary {
                        target: targets.pop().unwrap(),
                    });
            }
            Effect::TutorLibrary(TutorLibrary {
                destination,
                reveal,
                ..
            }) => {
                if reveal {
                    for target in targets.iter() {
                        let ActiveTarget::Library { id } = target else {
                            unreachable!()
                        };

                        self.settled_effects
                            .push_front(ActionResult::RevealCard(*id));
                    }
                }

                match destination {
                    Destination::Hand => {
                        for target in targets {
                            let ActiveTarget::Library { id } = target else {
                                unreachable!()
                            };
                            self.settled_effects
                                .push_front(ActionResult::MoveToHandFromLibrary(id));
                        }
                    }
                    Destination::TopOfLibrary => {
                        for target in targets {
                            let ActiveTarget::Library { id } = target else {
                                unreachable!()
                            };
                            self.settled_effects
                                .push_front(ActionResult::MoveFromLibraryToTopOfLibrary(id));
                        }
                    }
                    Destination::Battlefield { enters_tapped } => {
                        for target in targets {
                            let ActiveTarget::Library { id } = target else {
                                unreachable!()
                            };
                            self.settled_effects.push_front(
                                ActionResult::AddToBattlefieldFromLibrary {
                                    card: id,
                                    enters_tapped,
                                },
                            );
                        }
                    }
                }
            }
            Effect::TargetGainsCounters(counter) => {
                for target in targets.into_iter() {
                    let target = match target {
                        ActiveTarget::Battlefield { id } => id,
                        ActiveTarget::Graveyard { id } => id,
                        _ => unreachable!(),
                    };

                    self.settled_effects.push_front(ActionResult::AddCounters {
                        source: self.source.unwrap().card(db),
                        target,
                        counter,
                    });
                }
            }
            Effect::ControllerDrawCards(count) => {
                self.settled_effects.push_front(ActionResult::DrawCards {
                    target: self.source.unwrap().card(db).controller(db),
                    count,
                });
            }

            Effect::Equip(_) => unreachable!(),
            Effect::RevealEachTopOfLibrary(_) => unreachable!(),
            Effect::ReturnSelfToHand => unreachable!(),
            Effect::CreateToken(_) => unreachable!(),
            Effect::CounterSpell { .. } => unreachable!(),
            Effect::BattlefieldModifier(_) => unreachable!(),
            Effect::ControllerLosesLife(_) => unreachable!(),
            Effect::UntapThis => unreachable!(),
            Effect::Cascade => unreachable!(),
            Effect::UntapTarget => unreachable!(),
        }
    }

    pub(crate) fn can_cancel(&self) -> bool {
        self.is_empty() || !self.applied
    }
}
