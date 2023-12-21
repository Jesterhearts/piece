use std::collections::{HashSet, VecDeque};

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::{
    abilities::GainMana,
    battlefield::{ActionResult, Battlefield},
    controller::ControllerRestriction,
    effects::{DealDamage, Destination, Effect, Mill, TutorLibrary},
    in_play::{AbilityId, AuraId, CardId, CastFrom, Database, ModifierId, OnBattlefield},
    mana::{Mana, ManaCost},
    player::{mana_pool::ManaSource, AllPlayers, Controller, Owner},
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooseTargets {
    target_source: TargetSource,
    valid_targets: Vec<ActiveTarget>,
    chosen: IndexMap<usize, usize>,
    skipping_remainder: bool,
}

impl ChooseTargets {
    pub fn new(target_source: TargetSource, valid_targets: Vec<ActiveTarget>) -> Self {
        Self {
            target_source,
            valid_targets,
            chosen: Default::default(),
            skipping_remainder: false,
        }
    }

    pub fn recompute_targets(
        &mut self,
        db: &mut Database,
        source: Source,
        already_chosen: &HashSet<ActiveTarget>,
    ) {
        let card = source.card(db);
        let controller = card.controller(db);
        match &self.target_source {
            TargetSource::Effect(effect) => {
                self.valid_targets =
                    card.targets_for_effect(db, controller, effect, already_chosen);
            }
            TargetSource::Aura(_) => {
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

    pub fn into_effect(self) -> TargetSource {
        self.target_source
    }

    pub fn into_chosen_targets_and_effect(self) -> (Vec<ActiveTarget>, TargetSource) {
        let mut results = vec![];
        for choice in self
            .chosen
            .into_iter()
            .flat_map(|(choice, count)| std::iter::repeat(choice).take(count))
        {
            results.push(self.valid_targets[choice]);
        }

        (results, self.target_source)
    }

    pub fn into_chosen_targets(self) -> Vec<ActiveTarget> {
        self.into_chosen_targets_and_effect().0
    }

    pub fn chosen_targets_count(&self) -> usize {
        self.chosen.values().sum()
    }

    pub fn choices_complete(&self) -> bool {
        self.chosen_targets_count() >= self.target_source.wants_targets()
            || self.chosen_targets_count() >= self.valid_targets.len()
            || (self.can_skip() && self.skipping_remainder)
    }

    pub fn can_skip(&self) -> bool {
        self.chosen_targets_count() >= self.target_source.needs_targets()
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
pub struct TapPermanent {
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: Option<CardId>,
}

impl TapPermanent {
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
    paid: IndexMap<ManaCost, IndexMap<Mana, IndexMap<Option<ManaSource>, usize>>>,
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
                        let paid = paid
                            .values()
                            .flat_map(|sourced| sourced.values())
                            .sum::<usize>();
                        paid < required
                    })
                    .unwrap_or(true)
            })
            .map(|(paying, _)| *paying);
        unpaid
    }

    pub fn first_unpaid(&self) -> Option<ManaCost> {
        self.first_unpaid_x_always_unpaid()
            .filter(|unpaid| !matches!(unpaid, ManaCost::X))
    }

    pub fn paid(&self) -> bool {
        self.first_unpaid().is_none()
    }

    pub fn paying(&self) -> (Vec<Mana>, Vec<Option<ManaSource>>) {
        let mut mana_paid = vec![];
        let mut mana_source = vec![];
        for paid in self.paid.values() {
            for (mana, source) in paid.iter() {
                for (source, count) in source.iter() {
                    for _ in 0..*count {
                        mana_paid.push(*mana);
                        mana_source.push(*source)
                    }
                }
            }
        }

        (mana_paid, mana_source)
    }

    fn description(&self) -> String {
        match self.first_unpaid_x_always_unpaid().unwrap() {
            ManaCost::Generic(_) => "generic mana".to_string(),
            ManaCost::X => "X".to_string(),
            _ => "paying mana".to_string(),
        }
    }

    fn x_is(&self) -> Option<usize> {
        self.paid.get(&ManaCost::X).map(|paid| {
            paid.values()
                .flat_map(|sourced| sourced.values())
                .sum::<usize>()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayCost {
    SacrificePermanent(SacrificePermanent),
    TapPermanent(TapPermanent),
    SpendMana(SpendMana),
}

impl PayCost {
    fn autopay(&self, all_players: &AllPlayers, player: Owner) -> bool {
        match self {
            PayCost::SacrificePermanent(_) => false,
            PayCost::TapPermanent(_) => false,
            PayCost::SpendMana(spend) => {
                if let Some(first_unpaid) = spend.first_unpaid_x_always_unpaid() {
                    let (mana, source) = spend.paying();
                    let pool_post_pay = all_players[player].pool_post_pay(&mana, &source).unwrap();
                    match first_unpaid {
                        ManaCost::X | ManaCost::Generic(_) => return false,
                        unpaid => {
                            if !pool_post_pay.can_spend(unpaid, None) {
                                return false;
                            }
                        }
                    }
                }

                true
            }
        }
    }

    fn choice_optional(&self, all_players: &AllPlayers, player: Owner) -> bool {
        match self {
            PayCost::SacrificePermanent(_) => false,
            PayCost::TapPermanent(_) => false,
            PayCost::SpendMana(spend) => {
                let (mana, source) = spend.paying();
                if let Some(pool_post_pay) = all_players[player].pool_post_pay(&mana, &source) {
                    let first_unpaid = spend.first_unpaid_x_always_unpaid().unwrap();
                    match first_unpaid {
                        ManaCost::Generic(_) => true,
                        ManaCost::X => true,
                        unpaid => pool_post_pay.can_spend(unpaid, None),
                    }
                } else {
                    false
                }
            }
        }
    }

    fn paid(&self) -> bool {
        match self {
            PayCost::SacrificePermanent(sac) => sac.chosen.is_some(),
            PayCost::TapPermanent(tap) => tap.chosen.is_some(),
            PayCost::SpendMana(spend) => spend.paid(),
        }
    }

    fn options(
        &mut self,
        db: &mut Database,
        all_players: &AllPlayers,
        source: Source,
        all_targets: &HashSet<ActiveTarget>,
    ) -> Vec<(usize, String)> {
        let player = source.card(db).controller(db);
        self.compute_targets(db, source, all_targets);

        match self {
            PayCost::SacrificePermanent(sac) => sac
                .valid_targets
                .iter()
                .enumerate()
                .map(|(idx, target)| (idx, format!("{} - ({})", target.name(db), target.id(db),)))
                .collect_vec(),
            PayCost::TapPermanent(tap) => tap
                .valid_targets
                .iter()
                .enumerate()
                .map(|(idx, target)| (idx, format!("{} - ({})", target.name(db), target.id(db),)))
                .collect_vec(),
            PayCost::SpendMana(spend) => {
                let (mana, source) = spend.paying();
                let pool_post_paid = all_players[player].pool_post_pay(&mana, &source);
                if pool_post_paid.is_none() || pool_post_paid.as_ref().unwrap().max().is_none() {
                    return vec![];
                }
                let pool_post_paid = pool_post_paid.unwrap();

                match spend.first_unpaid_x_always_unpaid() {
                    Some(ManaCost::Generic(_) | ManaCost::X) => pool_post_paid
                        .available_pool_display()
                        .into_iter()
                        .enumerate()
                        .collect_vec(),
                    _ => vec![],
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
        match self {
            PayCost::SacrificePermanent(sac) => {
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
            PayCost::TapPermanent(tap) => {
                let card = source.card(db);
                let controller = card.controller(db);
                let valid_targets = controller
                    .get_cards::<OnBattlefield>(db)
                    .into_iter()
                    .filter(|target| {
                        !already_chosen.contains(&ActiveTarget::Battlefield { id: *target })
                            && !target.tapped(db)
                            && target.passes_restrictions(
                                db,
                                card,
                                ControllerRestriction::You,
                                &tap.restrictions,
                            )
                    })
                    .collect_vec();
                tap.valid_targets = valid_targets;
            }
            PayCost::SpendMana(_) => {}
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
            PayCost::TapPermanent(TapPermanent {
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
                    let (mana, source) = spend.paying();
                    let mut pool_post_pay =
                        all_players[player].pool_post_pay(&mana, &source).unwrap();
                    let Some(first_unpaid) = spend.first_unpaid() else {
                        return self.paid();
                    };

                    if pool_post_pay.can_spend(first_unpaid, None) {
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
                                    let (_, source) = pool_post_pay.spend(max, None);
                                    *spend
                                        .paid
                                        .entry(first_unpaid)
                                        .or_default()
                                        .entry(max)
                                        .or_default()
                                        .entry(source)
                                        .or_default() += 1;
                                }

                                return !matches!(
                                    spend.first_unpaid_x_always_unpaid(),
                                    Some(ManaCost::X)
                                );
                            }
                            ManaCost::X => unreachable!(),
                        };
                        let (_, source) = pool_post_pay.spend(mana, None);
                        *spend
                            .paid
                            .entry(first_unpaid)
                            .or_default()
                            .entry(mana)
                            .or_default()
                            .entry(source)
                            .or_default() += 1;
                        return true;
                    } else {
                        return false;
                    }
                }

                let (mana, sources) = spend.paying();
                if let Some((_, mana, source)) = all_players[player]
                    .pool_post_pay(&mana, &sources)
                    .unwrap()
                    .available_mana()
                    .filter(|(count, _, _)| *count > 0)
                    .nth(choice.unwrap())
                {
                    let cost = spend.first_unpaid_x_always_unpaid().unwrap();
                    *spend
                        .paid
                        .entry(cost)
                        .or_default()
                        .entry(mana)
                        .or_default()
                        .entry(source)
                        .or_default() += 1;

                    let (mana, sources) = spend.paying();
                    if all_players[player].can_spend_mana(&mana, &sources) {
                        !matches!(spend.first_unpaid_x_always_unpaid(), Some(ManaCost::X))
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    fn results(&self, db: &Database, source: Source) -> ActionResult {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent { chosen, .. }) => {
                ActionResult::PermanentToGraveyard(chosen.unwrap())
            }
            PayCost::TapPermanent(TapPermanent { chosen, .. }) => {
                ActionResult::TapPermanent(chosen.unwrap())
            }
            PayCost::SpendMana(spend) => {
                let (mana, sources) = spend.paying();
                ActionResult::SpendMana {
                    card: source.card(db),
                    mana,
                    sources,
                }
            }
        }
    }

    fn description(&self) -> String {
        match self {
            PayCost::SacrificePermanent(_) => "sacrificing a permanent".to_string(),
            PayCost::TapPermanent(_) => "tapping a permanent".to_string(),
            PayCost::SpendMana(spend) => spend.description(),
        }
    }

    fn x_is(&self) -> Option<usize> {
        match self {
            PayCost::SacrificePermanent(_) | PayCost::TapPermanent(_) => None,
            PayCost::SpendMana(spend) => spend.x_is(),
        }
    }

    fn target(&self) -> Option<ActiveTarget> {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent { chosen, .. }) => {
                chosen.map(|id| ActiveTarget::Battlefield { id })
            }
            PayCost::TapPermanent(TapPermanent { chosen, .. }) => {
                chosen.map(|id| ActiveTarget::Battlefield { id })
            }
            PayCost::SpendMana(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChoosingScry {
    cards: VecDeque<CardId>,
    cards_on_bottom: Vec<CardId>,
    cards_on_top: Vec<CardId>,
    placing_on_top: bool,
}
impl ChoosingScry {
    fn choose(&mut self, choice: Option<usize>) -> bool {
        debug!("Choosing to scry to top = {}", self.placing_on_top);
        if choice.is_none() && !self.placing_on_top {
            self.placing_on_top = true;
            return false;
        } else if choice.is_none() {
            for card in self.cards.drain(..) {
                self.cards_on_top.push(card);
            }
            return true;
        }

        if self.placing_on_top {
            let card = self.cards.remove(choice.unwrap()).unwrap();
            self.cards_on_top.push(card);
        } else {
            let card = self.cards.remove(choice.unwrap()).unwrap();
            self.cards_on_bottom.push(card);
        }

        self.cards.is_empty()
    }

    fn is_empty(&self) -> bool {
        self.cards.is_empty() && self.cards_on_bottom.is_empty() && self.cards_on_bottom.is_empty()
    }

    fn options(&self, db: &Database) -> Vec<(usize, String)> {
        self.cards
            .iter()
            .map(|card| card.name(db))
            .enumerate()
            .collect_vec()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizingStack {
    entries: Vec<StackEntry>,
    choices: IndexSet<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaringAttackers {
    candidates: Vec<CardId>,
    choices: IndexSet<usize>,
    targets: Vec<Owner>,
    valid_targets: Vec<Owner>,
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PendingResults {
    source: Option<Source>,

    declare_attackers: Option<DeclaringAttackers>,
    choose_modes: VecDeque<()>,
    choose_targets: VecDeque<ChooseTargets>,
    choosing_scry: ChoosingScry,
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
    discovering: bool,

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

    pub fn discovering(&mut self) {
        self.discovering = true;
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

    pub(crate) fn push_choose_scry(&mut self, cards: Vec<CardId>) {
        self.choosing_scry.cards.extend(cards);
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

    pub fn set_declare_attackers(
        &mut self,
        db: &mut Database,
        all_players: &AllPlayers,
        attacker: Owner,
    ) {
        let mut players = all_players.all_players();
        players.retain(|player| *player != attacker);
        debug!("Attacking {:?}", players);
        // TODO goad, etc.
        self.declare_attackers = Some(DeclaringAttackers {
            candidates: attacker
                .get_cards::<OnBattlefield>(db)
                .into_iter()
                .filter(|card| card.can_attack(db))
                .collect_vec(),
            choices: IndexSet::default(),
            targets: vec![],
            valid_targets: players,
        });
    }

    pub fn choices_optional(&self, db: &Database, all_players: &AllPlayers) -> bool {
        if self.declare_attackers.is_some() {
            true
        } else if self.choose_modes.front().is_some() {
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

    pub fn options(&mut self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
        if let Some(declaring) = self.declare_attackers.as_ref() {
            if declaring.choices.len() == declaring.targets.len() {
                declaring
                    .candidates
                    .iter()
                    .map(|card| card.name(db))
                    .enumerate()
                    .filter(|(idx, _)| !declaring.choices.contains(idx))
                    .collect_vec()
            } else {
                declaring
                    .valid_targets
                    .iter()
                    .map(|player| all_players[*player].name.clone())
                    .enumerate()
                    .collect_vec()
            }
        } else if self.choose_modes.front().is_some() {
            self.source.unwrap().mode_options(db)
        } else if let Some(choosing) = self.choose_targets.front() {
            choosing.options(db, all_players)
        } else if let Some(choosing) = self.pay_costs.front_mut() {
            choosing.options(
                db,
                all_players,
                self.source.unwrap(),
                &self.all_chosen_targets,
            )
        } else if !self.choosing_scry.cards.is_empty() {
            self.choosing_scry.options(db)
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
        if self.declare_attackers.is_some() {
            "attackers".to_string()
        } else if self.choose_modes.front().is_some() {
            "mode".to_string()
        } else if self.choose_targets.front().is_some() {
            "targets".to_string()
        } else if let Some(pay) = self.pay_costs.front() {
            pay.description()
        } else if !self.choosing_scry.cards.is_empty() {
            if self.choosing_scry.placing_on_top {
                "placing on top of your library".to_string()
            } else {
                "placing on the bottom of your library".to_string()
            }
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

        if self.declare_attackers.is_none()
            && self.choose_modes.is_empty()
            && self.choose_targets.is_empty()
            && self.pay_costs.is_empty()
            && self.choosing_scry.is_empty()
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
                let mana_source = id.mana_source(db);
                if let Some(mana) = id.gain_mana_ability(db) {
                    match mana.gain {
                        GainMana::Specific { gains } => {
                            self.settled_effects.push_back(ActionResult::GainMana {
                                gain: gains,
                                target: controller,
                                source: mana_source,
                            })
                        }
                        GainMana::Choice { choices } => {
                            let option = self.chosen_modes.pop().unwrap();
                            self.settled_effects.push_back(ActionResult::GainMana {
                                gain: choices[option].clone(),
                                target: controller,
                                source: mana_source,
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
                choice.recompute_targets(db, self.source.unwrap(), &self.all_chosen_targets);
            }
        }

        if let Some(declaring) = self.declare_attackers.as_mut() {
            if let Some(choice) = choice {
                if declaring.choices.len() == declaring.targets.len() {
                    declaring.choices.insert(choice);
                } else {
                    declaring.targets.push(declaring.valid_targets[choice]);
                }
                ResolutionResult::PendingChoice
            } else if declaring.choices.len() == declaring.targets.len() {
                self.settled_effects
                    .push_front(ActionResult::DeclareAttackers {
                        attackers: declaring
                            .choices
                            .iter()
                            .map(|choice| declaring.candidates[*choice])
                            .collect_vec(),
                        targets: declaring.targets.clone(),
                    });
                self.declare_attackers = None;
                ResolutionResult::TryAgain
            } else {
                ResolutionResult::PendingChoice
            }
        } else if let Some(choosing) = self.choose_modes.pop_front() {
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
                            TargetSource::Effect(effect) => {
                                self.push_effect_results(db, player, effect, choices.clone());
                            }
                            TargetSource::Aura(aura) => {
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
                                    TargetSource::Effect(effect) => {
                                        self.push_effect_results(
                                            db,
                                            player,
                                            effect,
                                            choices.clone(),
                                        );
                                    }
                                    TargetSource::Aura(aura) => self.settled_effects.push_back(
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
                } else {
                    self.choose_targets.push_front(choosing);
                }
                ResolutionResult::TryAgain
            } else {
                self.choose_targets.push_front(choosing);
                ResolutionResult::PendingChoice
            }
        } else if let Some(mut pay) = self.pay_costs.pop_front() {
            let player = self.source.unwrap().card(db).controller(db);
            pay.compute_targets(db, self.source.unwrap(), &self.all_chosen_targets);
            if pay.choose_pay(all_players, player.into(), &self.all_chosen_targets, choice) {
                if pay.paid() {
                    self.x_is = pay.x_is();
                    if let Some(target) = pay.target() {
                        self.all_chosen_targets.insert(target);
                    }
                    self.settled_effects
                        .push_back(pay.results(db, self.source.unwrap()));
                } else {
                    self.pay_costs.push_front(pay);
                }
                ResolutionResult::TryAgain
            } else {
                self.pay_costs.push_front(pay);
                ResolutionResult::PendingChoice
            }
        } else if !self.choosing_scry.is_empty() {
            if self.choosing_scry.choose(choice) {
                for card in self.choosing_scry.cards_on_bottom.drain(..) {
                    all_players[self.source.unwrap().card(db).controller(db)]
                        .deck
                        .place_on_bottom(db, card);
                }

                for card in self.choosing_scry.cards_on_top.drain(..) {
                    all_players[self.source.unwrap().card(db).controller(db)]
                        .deck
                        .place_on_top(db, card);
                }
                ResolutionResult::TryAgain
            } else {
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
                if self.discovering {
                    let card = *self.choosing_to_cast.iter().exactly_one().unwrap();
                    card.move_to_hand(db);
                }
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
        self.choosing_scry.cards.extend(results.choosing_scry.cards);
        self.choose_modes.extend(results.choose_modes);
        self.choose_targets.extend(results.choose_targets);
        self.pay_costs.extend(results.pay_costs);
        self.choosing_to_cast.extend(results.choosing_to_cast);
        self.settled_effects.extend(results.settled_effects);

        self.declare_attackers = results.declare_attackers;
        self.organizing_stack = results.organizing_stack;
        self.cast_from = results.cast_from;
        self.apply_in_stages = results.apply_in_stages;
        self.add_to_stack = results.add_to_stack;
        self.paying_costs = results.paying_costs;
        self.discovering = results.discovering;
    }

    pub fn is_empty(&self) -> bool {
        self.declare_attackers.is_none()
            && self.choose_modes.is_empty()
            && self.choose_targets.is_empty()
            && self.pay_costs.is_empty()
            && self.choosing_to_cast.is_empty()
            && self.choosing_scry.is_empty()
            && self.organizing_stack.is_none()
            && self.settled_effects.is_empty()
    }

    pub fn only_immediate_results(&self, db: &Database, all_players: &AllPlayers) -> bool {
        (self.choosing_to_cast.is_empty()
            && self.choose_modes.is_empty()
            && self.choosing_scry.cards.is_empty()
            && self.declare_attackers.is_none()
            && self.organizing_stack.is_none())
            && ((self.choose_targets.is_empty() && self.pay_costs.is_empty())
                || (self
                    .choose_targets
                    .iter()
                    .all(|choose| choose.valid_targets.is_empty())
                    && self.pay_costs.iter().all(|pay| {
                        pay.autopay(all_players, self.source.unwrap().card(db).owner(db))
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
            Effect::Scry(count) => {
                self.settled_effects
                    .push_front(ActionResult::Scry(self.source.unwrap().card(db), count));
            }
            Effect::GainLife(count) => {
                self.settled_effects.push_front(ActionResult::GainLife {
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
            Effect::Discover(_) => unreachable!(),
            Effect::UntapTarget => unreachable!(),
            Effect::ForEachManaOfSource(_) => unreachable!(),
            Effect::Craft(_) => unreachable!(),
        }
    }

    pub fn can_cancel(&self) -> bool {
        self.is_empty() || !self.applied
    }
}
