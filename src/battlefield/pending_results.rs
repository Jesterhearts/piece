use std::{
    collections::{HashSet, VecDeque},
    fmt::Debug,
};

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::{
    abilities::GainMana,
    battlefield::{ActionResult, Battlefield},
    controller::ControllerRestriction,
    effects::{Effect, EffectDuration},
    in_play::{AbilityId, AuraId, CardId, CastFrom, Database, OnBattlefield},
    mana::{Mana, ManaCost},
    player::{
        mana_pool::{ManaSource, SpendReason},
        AllPlayers, Controller, Owner,
    },
    stack::{ActiveTarget, Stack, StackEntry},
    targets::Restriction,
    turns::Turn,
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
    pub fn card(&self, db: &Database) -> CardId {
        match self {
            Source::Card(id) => *id,
            Source::Ability(id) => id.source(db),
        }
    }

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

#[derive(Debug, Clone)]
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
                self.valid_targets = effect.valid_targets(db, card, controller, already_chosen);
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

    fn is_empty(&self) -> bool {
        self.valid_targets.is_empty()
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
pub struct ExilePermanentsCmcX {
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: IndexSet<CardId>,
    target: usize,
}

impl ExilePermanentsCmcX {
    pub fn new(restrictions: Vec<Restriction>) -> Self {
        Self {
            restrictions,
            valid_targets: Default::default(),
            chosen: Default::default(),
            target: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendMana {
    paying: IndexMap<ManaCost, usize>,
    paid: IndexMap<ManaCost, IndexMap<Mana, IndexMap<ManaSource, usize>>>,
    reason: SpendReason,
}

impl SpendMana {
    pub fn new(mut mana: Vec<ManaCost>, reason: SpendReason) -> Self {
        mana.sort();

        let mut paying = IndexMap::default();
        for cost in mana {
            *paying.entry(cost).or_default() += 1;
        }
        let mut paid = IndexMap::default();
        paid.entry(ManaCost::X).or_default();
        paid.entry(ManaCost::TwoX).or_default();

        Self {
            paying,
            paid,
            reason,
        }
    }

    pub fn first_unpaid_x_always_unpaid(&self) -> Option<ManaCost> {
        let unpaid = self
            .paying
            .iter()
            .find(|(paying, required)| {
                let required = match paying {
                    ManaCost::Generic(count) => *count,
                    ManaCost::X => usize::MAX,
                    ManaCost::TwoX => usize::MAX,
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
            .filter(|unpaid| !matches!(unpaid, ManaCost::X | ManaCost::TwoX))
    }

    pub fn paid(&self) -> bool {
        self.first_unpaid().is_none()
    }

    pub fn paying(&self) -> (Vec<Mana>, Vec<ManaSource>) {
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
        if let Some(first_unpaid) = self.first_unpaid_x_always_unpaid() {
            match first_unpaid {
                ManaCost::Generic(_) => "generic mana".to_string(),
                ManaCost::X => "X".to_string(),
                _ => "paying mana".to_string(),
            }
        } else {
            String::default()
        }
    }

    fn x_is(&self) -> Option<usize> {
        self.paid
            .get(&ManaCost::X)
            .map(|paid| {
                paid.values()
                    .flat_map(|sourced| sourced.values())
                    .sum::<usize>()
            })
            .filter(|paid| *paid != 0)
            .or_else(|| {
                self.paid
                    .get(&ManaCost::TwoX)
                    .map(|paid| {
                        paid.values()
                            .flat_map(|sourced| sourced.values())
                            .sum::<usize>()
                            / 2
                    })
                    .filter(|paid| *paid != 0)
            })
    }

    fn is_empty(&self) -> bool {
        !self.paying.values().any(|v| *v != 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayCost {
    SacrificePermanent(SacrificePermanent),
    TapPermanent(TapPermanent),
    SpendMana(SpendMana),
    ExilePermanentsCmcX(ExilePermanentsCmcX),
}

impl PayCost {
    fn autopay(&self, db: &Database, all_players: &AllPlayers, player: Owner) -> bool {
        match self {
            PayCost::SacrificePermanent(_) => false,
            PayCost::TapPermanent(_) => false,
            PayCost::ExilePermanentsCmcX(_) => false,
            PayCost::SpendMana(spend) => {
                debug!("Checking autopay: {:?}", spend,);
                if let Some(first_unpaid) = spend.first_unpaid_x_always_unpaid() {
                    debug!("first unpaid {:?}", first_unpaid,);
                    let (mana, source) = spend.paying();
                    match first_unpaid {
                        ManaCost::TwoX | ManaCost::X | ManaCost::Generic(_) => return false,
                        unpaid => {
                            let pool_post_pay = all_players[player]
                                .pool_post_pay(db, &mana, &source, spend.reason)
                                .unwrap();
                            if !pool_post_pay.can_spend(db, unpaid, ManaSource::Any, spend.reason) {
                                return false;
                            }
                        }
                    }
                }

                true
            }
        }
    }

    fn choice_optional(&self, db: &Database, all_players: &AllPlayers, player: Owner) -> bool {
        match self {
            PayCost::SacrificePermanent(_) => false,
            PayCost::TapPermanent(_) => false,
            PayCost::ExilePermanentsCmcX(_) => true,
            PayCost::SpendMana(spend) => {
                let (mana, source) = spend.paying();
                if let Some(pool_post_pay) =
                    all_players[player].pool_post_pay(db, &mana, &source, spend.reason)
                {
                    if let Some(first_unpaid) = spend.first_unpaid_x_always_unpaid() {
                        match first_unpaid {
                            ManaCost::Generic(_) => true,
                            ManaCost::X => true,
                            ManaCost::TwoX => {
                                spend
                                    .paid
                                    .get(&ManaCost::TwoX)
                                    .iter()
                                    .flat_map(|i| i.values())
                                    .flat_map(|i| i.values())
                                    .sum::<usize>()
                                    % 2
                                    == 0
                            }
                            unpaid => {
                                pool_post_pay.can_spend(db, unpaid, ManaSource::Any, spend.reason)
                            }
                        }
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
        }
    }

    fn paid(&self, db: &Database) -> bool {
        match self {
            PayCost::SacrificePermanent(sac) => sac.chosen.is_some(),
            PayCost::TapPermanent(tap) => tap.chosen.is_some(),
            PayCost::SpendMana(spend) => spend.paid(),
            PayCost::ExilePermanentsCmcX(exile) => {
                exile
                    .chosen
                    .iter()
                    .map(|chosen| chosen.cost(db).cmc())
                    .sum::<usize>()
                    >= exile.target
            }
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
                let (mana, sources) = spend.paying();
                let pool_post_paid =
                    all_players[player].pool_post_pay(db, &mana, &sources, spend.reason);
                if pool_post_paid.is_none()
                    || pool_post_paid
                        .as_ref()
                        .unwrap()
                        .max(db, spend.reason)
                        .is_none()
                {
                    return vec![];
                }
                let pool_post_paid = pool_post_paid.unwrap();

                match spend.first_unpaid_x_always_unpaid() {
                    Some(ManaCost::Generic(_) | ManaCost::X | ManaCost::TwoX) => pool_post_paid
                        .available_pool_display()
                        .into_iter()
                        .enumerate()
                        .collect_vec(),
                    _ => vec![],
                }
            }
            PayCost::ExilePermanentsCmcX(exile) => exile
                .valid_targets
                .iter()
                .enumerate()
                .filter(|(_, chosen)| !exile.chosen.contains(*chosen))
                .map(|(idx, target)| (idx, target.name(db)))
                .collect_vec(),
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
            PayCost::ExilePermanentsCmcX(exile) => {
                exile.target = already_chosen
                    .iter()
                    .map(|target| target.id().unwrap().cost(db).cmc())
                    .sum::<usize>();

                let card = source.card(db);
                let controller = card.controller(db);
                let valid_targets = controller
                    .get_cards::<OnBattlefield>(db)
                    .into_iter()
                    .filter(|target| {
                        target.passes_restrictions(
                            db,
                            card,
                            ControllerRestriction::You,
                            &exile.restrictions,
                        )
                    })
                    .collect_vec();

                exile.valid_targets = valid_targets;
            }
        }
    }

    fn choose_pay(
        &mut self,
        db: &Database,
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
            PayCost::ExilePermanentsCmcX(ExilePermanentsCmcX {
                valid_targets,
                chosen,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice];
                    if !all_targets.contains(&ActiveTarget::Battlefield { id: target }) {
                        chosen.insert(target);
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            PayCost::SpendMana(spend) => {
                if choice.is_none() {
                    if spend
                        .paid
                        .entry(ManaCost::TwoX)
                        .or_default()
                        .values()
                        .flat_map(|i| i.values())
                        .sum::<usize>()
                        % 2
                        != 0
                    {
                        return false;
                    }

                    let (mana, source) = spend.paying();
                    let mut pool_post_pay = all_players[player]
                        .pool_post_pay(db, &mana, &source, spend.reason)
                        .unwrap();
                    let Some(first_unpaid) = spend.first_unpaid() else {
                        return true;
                    };

                    if pool_post_pay.can_spend(db, first_unpaid, ManaSource::Any, spend.reason) {
                        let mana = match first_unpaid {
                            ManaCost::White => Mana::White,
                            ManaCost::Blue => Mana::Blue,
                            ManaCost::Black => Mana::Black,
                            ManaCost::Red => Mana::Red,
                            ManaCost::Green => Mana::Green,
                            ManaCost::Colorless => Mana::Colorless,
                            ManaCost::Generic(count) => {
                                for _ in 0..count {
                                    let max = pool_post_pay.max(db, spend.reason).unwrap();
                                    let (_, source) =
                                        pool_post_pay.spend(db, max, ManaSource::Any, spend.reason);
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
                            ManaCost::TwoX => unreachable!(),
                        };
                        let (_, source) =
                            pool_post_pay.spend(db, mana, ManaSource::Any, spend.reason);
                        *spend
                            .paid
                            .entry(first_unpaid)
                            .or_default()
                            .entry(mana)
                            .or_default()
                            .entry(source)
                            .or_default() += 1;

                        return !matches!(
                            spend.first_unpaid_x_always_unpaid(),
                            Some(ManaCost::X | ManaCost::TwoX)
                        );
                    } else {
                        return false;
                    }
                }

                let (mana, sources) = spend.paying();
                if let Some((_, mana, source, _)) = all_players[player]
                    .pool_post_pay(db, &mana, &sources, spend.reason)
                    .unwrap()
                    .available_mana()
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
                    if all_players[player].can_spend_mana(db, &mana, &sources, spend.reason) {
                        !matches!(
                            spend.first_unpaid_x_always_unpaid(),
                            Some(ManaCost::X | ManaCost::TwoX)
                        )
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    fn results(&self, db: &Database, source: Source) -> Vec<ActionResult> {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent { chosen, .. }) => {
                vec![ActionResult::PermanentToGraveyard(chosen.unwrap())]
            }
            PayCost::TapPermanent(TapPermanent { chosen, .. }) => {
                vec![ActionResult::TapPermanent(chosen.unwrap())]
            }
            PayCost::ExilePermanentsCmcX(exile) => {
                let mut results = vec![];
                for target in exile.chosen.iter() {
                    results.push(ActionResult::ExileTarget {
                        source: source.card(db),
                        target: ActiveTarget::Battlefield { id: *target },
                        duration: EffectDuration::Permanently,
                    });
                }
                results
            }
            PayCost::SpendMana(spend) => {
                let (mana, sources) = spend.paying();
                vec![ActionResult::SpendMana {
                    card: source.card(db),
                    mana,
                    sources,
                    reason: spend.reason,
                }]
            }
        }
    }

    fn description(&self) -> String {
        match self {
            PayCost::SacrificePermanent(_) => "sacrificing a permanent".to_string(),
            PayCost::TapPermanent(_) => "tapping a permanent".to_string(),
            PayCost::SpendMana(spend) => spend.description(),
            PayCost::ExilePermanentsCmcX(_) => "exiling a permanent".to_string(),
        }
    }

    fn x_is(&self) -> Option<usize> {
        match self {
            PayCost::SacrificePermanent(_) | PayCost::TapPermanent(_) => None,
            PayCost::SpendMana(spend) => spend.x_is(),
            PayCost::ExilePermanentsCmcX(exile) => Some(exile.target),
        }
    }

    fn chosen_targets(&self) -> Vec<ActiveTarget> {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent { chosen, .. }) => chosen
                .map(|id| ActiveTarget::Battlefield { id })
                .into_iter()
                .collect_vec(),
            PayCost::TapPermanent(TapPermanent { chosen, .. }) => chosen
                .map(|id| ActiveTarget::Battlefield { id })
                .into_iter()
                .collect_vec(),
            PayCost::SpendMana(_) => vec![],
            PayCost::ExilePermanentsCmcX(exile) => exile
                .chosen
                .iter()
                .map(|chosen| ActiveTarget::Battlefield { id: *chosen })
                .collect_vec(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            PayCost::SacrificePermanent(_) => false,
            PayCost::TapPermanent(_) => false,
            PayCost::SpendMana(spend) => spend.is_empty(),
            PayCost::ExilePermanentsCmcX(_) => false,
        }
    }

    fn targets(&self) -> Vec<CardId> {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent { valid_targets, .. }) => {
                valid_targets.clone()
            }
            PayCost::TapPermanent(TapPermanent { valid_targets, .. }) => valid_targets.clone(),
            PayCost::SpendMana(_) => vec![],
            PayCost::ExilePermanentsCmcX(ExilePermanentsCmcX { valid_targets, .. }) => {
                valid_targets.clone()
            }
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
#[derive(Debug, Clone, Default)]
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

    pub fn push_invalid_target(&mut self, target: ActiveTarget) {
        self.all_chosen_targets.insert(target);
    }

    pub fn all_currently_targeted(&self) -> &HashSet<ActiveTarget> {
        &self.all_chosen_targets
    }

    pub fn push_choose_mode(&mut self) {
        self.choose_modes.push_back(());
    }

    pub fn push_choose_targets(&mut self, choice: ChooseTargets) {
        self.choose_targets.push_back(choice);
    }

    pub fn push_pay_costs(&mut self, pay: PayCost) {
        if !pay.is_empty() {
            self.pay_costs.push_back(pay);
        }
    }

    pub fn set_organize_stack(&mut self, db: &Database, mut entries: Vec<StackEntry>, turn: &Turn) {
        entries.sort_by_key(|e| {
            e.ty.source().controller(db) != Controller::from(turn.priority_player())
        });
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
        debug!(
            "Attacking {:?}",
            players
                .iter()
                .map(|player| all_players[*player].name.clone())
                .collect_vec()
        );
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
                cost.choice_optional(db, all_players, self.source.unwrap().card(db).owner(db))
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
        } else if !self.choose_targets.is_empty() {
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
        debug!("Choosing {:?} for {:?}", choice, self);

        if !self.apply_in_stages
            && self.choose_targets.iter().all(|targets| targets.is_empty())
            && !self.choose_targets.is_empty()
        {
            self.choose_targets.clear();
            return ResolutionResult::TryAgain;
        }
        if self.pay_costs.iter().all(|pay| pay.is_empty()) && !self.pay_costs.is_empty() {
            self.pay_costs.clear();
            return ResolutionResult::TryAgain;
        }

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
                }
                self.add_to_stack = false;
            } else if let Some(Source::Ability(id)) = self.source {
                let target = self.source.unwrap().card(db).controller(db);
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
                if declaring.candidates.is_empty() {
                    ResolutionResult::Complete
                } else {
                    if declaring.choices.len() == declaring.targets.len() {
                        if !declaring.choices.insert(choice) {
                            return ResolutionResult::Complete;
                        }
                    } else {
                        declaring.targets.push(declaring.valid_targets[choice]);
                    }
                    ResolutionResult::PendingChoice
                }
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

                    for target in choices.iter() {
                        if let ActiveTarget::Battlefield { id } = target {
                            if let Some(ward) = id.ward(db) {
                                self.push_pay_costs(PayCost::SpendMana(SpendMana::new(
                                    ward.mana_cost.clone(),
                                    SpendReason::Other,
                                )));
                            }
                        }
                    }

                    if !self.add_to_stack {
                        let player = self.source.unwrap().card(db).controller(db);
                        match effect_or_aura {
                            TargetSource::Effect(effect) => {
                                effect.push_behavior_with_targets(
                                    db,
                                    choices.clone(),
                                    false,
                                    self.source.unwrap().card(db),
                                    player,
                                    self,
                                );
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
                                        effect.push_behavior_with_targets(
                                            db,
                                            choices.clone(),
                                            false,
                                            self.source.unwrap().card(db),
                                            player,
                                            self,
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
            let targets = pay.targets();
            pay.compute_targets(db, self.source.unwrap(), &self.all_chosen_targets);
            if pay.targets() != targets {
                self.pay_costs.push_front(pay);
                return ResolutionResult::TryAgain;
            }

            if pay.choose_pay(
                db,
                all_players,
                player.into(),
                &self.all_chosen_targets,
                choice,
            ) {
                if pay.paid(db) {
                    self.x_is = pay.x_is();
                    debug!("X is {:?}", self.x_is);
                    for target in pay.chosen_targets() {
                        self.all_chosen_targets.insert(target);
                    }
                    self.settled_effects
                        .extend(pay.results(db, self.source.unwrap()));
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
            && ((self.choose_targets.iter().all(|targets| targets.is_empty())
                && self.pay_costs.iter().all(|pay| pay.is_empty()))
                || (self
                    .choose_targets
                    .iter()
                    .all(|choose| choose.valid_targets.is_empty())
                    && self.pay_costs.iter().all(|pay| {
                        pay.autopay(db, all_players, self.source.unwrap().card(db).owner(db))
                    })))
    }

    pub fn can_cancel(&self) -> bool {
        self.is_empty() || !self.applied
    }

    pub fn priority(&self, db: &Database, all_players: &AllPlayers, turn: &Turn) -> Owner {
        if let Some(attacking) = self.declare_attackers.as_ref() {
            let mut all_players = all_players
                .all_players()
                .into_iter()
                .collect::<HashSet<_>>();
            for target in attacking.valid_targets.iter() {
                all_players.remove(target);
            }

            return all_players.into_iter().exactly_one().unwrap();
        } else if let Some(organizing) = self.organizing_stack.as_ref() {
            if let Some(first) = organizing
                .entries
                .iter()
                .enumerate()
                .find(|(idx, _)| !organizing.choices.contains(idx))
            {
                return first.1.ty.source().controller(db).into();
            }
        }
        turn.priority_player()
    }
}
