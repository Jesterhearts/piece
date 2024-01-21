use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::{Database, ExileReason},
    log::LogId,
    pending_results::{Options, PendingResult, PendingResults},
    player::mana_pool::SpendReason,
    protogen::targets::Restriction,
    protogen::{
        cost::{cost_reducer::When, ManaCost},
        effects::{effect::Effect, Duration},
        ids::CardId,
        mana::Mana,
        mana::ManaSource,
    },
    stack::ActiveTarget,
};

#[derive(Debug)]
pub(crate) struct SacrificePermanent {
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: Option<CardId>,
}

impl SacrificePermanent {
    pub(crate) fn new(restrictions: Vec<Restriction>) -> Self {
        Self {
            restrictions,
            valid_targets: Default::default(),
            chosen: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TapPermanent {
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: Option<CardId>,
}

impl TapPermanent {
    pub(crate) fn new(restrictions: Vec<Restriction>) -> Self {
        Self {
            restrictions,
            valid_targets: Default::default(),
            chosen: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TapPermanentsPowerXOrMore {
    restrictions: Vec<Restriction>,
    x_is: usize,
    valid_targets: Vec<CardId>,
    chosen: IndexSet<CardId>,
}

impl TapPermanentsPowerXOrMore {
    pub(crate) fn new(restrictions: Vec<Restriction>, x_is: usize) -> Self {
        Self {
            restrictions,
            x_is,
            valid_targets: Default::default(),
            chosen: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ExilePermanentsCmcX {
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: IndexSet<CardId>,
    target: usize,
}

impl ExilePermanentsCmcX {
    pub(crate) fn new(restrictions: Vec<Restriction>) -> Self {
        Self {
            restrictions,
            valid_targets: Default::default(),
            chosen: Default::default(),
            target: 0,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ExileCards {
    reason: Option<ExileReason>,
    minimum: usize,
    maximum: usize,
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: IndexSet<CardId>,
}

impl ExileCards {
    pub(crate) fn new(
        reason: Option<ExileReason>,
        minimum: usize,
        maximum: usize,
        restrictions: Vec<Restriction>,
    ) -> Self {
        Self {
            reason,
            minimum,
            maximum,
            restrictions,
            valid_targets: Default::default(),
            chosen: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ExileCardsSharingType {
    reason: Option<ExileReason>,
    count: usize,
    chosen: IndexSet<CardId>,
    valid_targets: Vec<CardId>,
}

impl ExileCardsSharingType {
    pub(crate) fn new(reason: Option<ExileReason>, count: usize) -> Self {
        Self {
            reason,
            count,
            chosen: Default::default(),
            valid_targets: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct SpendMana {
    paying: IndexMap<ManaCost, usize>,
    paid: IndexMap<ManaCost, IndexMap<Mana, IndexMap<ManaSource, usize>>>,
    reason: SpendReason,
    reduced: bool,
}

impl SpendMana {
    pub(crate) fn new(
        mut mana: Vec<protobuf::EnumOrUnknown<ManaCost>>,
        reason: SpendReason,
    ) -> Self {
        mana.sort();

        let mut paying = IndexMap::default();
        for cost in mana {
            *paying.entry(cost.enum_value().unwrap()).or_default() += 1;
        }
        let mut paid = IndexMap::default();
        paid.entry(ManaCost::X).or_default();
        paid.entry(ManaCost::TWO_X).or_default();

        Self {
            paying,
            paid,
            reason,
            reduced: false,
        }
    }

    pub(crate) fn first_unpaid_x_always_unpaid(&self) -> Option<ManaCost> {
        let unpaid = self
            .paying
            .iter()
            .find(|(paying, required)| {
                let required = match paying {
                    ManaCost::X => usize::MAX,
                    ManaCost::TWO_X => usize::MAX,
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

    pub(crate) fn first_unpaid(&self) -> Option<ManaCost> {
        self.first_unpaid_x_always_unpaid()
            .filter(|unpaid| !matches!(unpaid, ManaCost::X | ManaCost::TWO_X))
    }

    pub(crate) fn paid(&self) -> bool {
        self.first_unpaid().is_none()
    }

    pub(crate) fn paying(&self) -> (Vec<Mana>, Vec<ManaSource>) {
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
                ManaCost::GENERIC => "generic mana".to_string(),
                ManaCost::X | ManaCost::TWO_X => "X".to_string(),
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
                    .get(&ManaCost::TWO_X)
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

#[derive(Debug)]
pub(crate) enum Cost {
    SacrificePermanent(SacrificePermanent),
    TapPermanent(TapPermanent),
    TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore),
    SpendMana(SpendMana),
    ExilePermanentsCmcX(ExilePermanentsCmcX),
    ExileCards(ExileCards),
    ExileCardsSharingType(ExileCardsSharingType),
}

impl Cost {
    fn paid(&self, db: &mut Database) -> bool {
        match self {
            Cost::SacrificePermanent(sac) => sac.chosen.is_some(),
            Cost::TapPermanent(tap) => tap.chosen.is_some(),
            Cost::TapPermanentsPowerXOrMore(tap) => {
                tap.chosen
                    .iter()
                    .map(|card| card.power(db).unwrap_or_default())
                    .sum::<i32>()
                    >= tap.x_is as i32
            }
            Cost::SpendMana(spend) => spend.paid(),
            Cost::ExilePermanentsCmcX(exile) => {
                exile
                    .chosen
                    .iter()
                    .map(|chosen| chosen.faceup_face(db).cost.cmc())
                    .sum::<usize>()
                    >= exile.target
            }
            Cost::ExileCards(exile) => exile.chosen.len() >= exile.minimum,
            Cost::ExileCardsSharingType(exile) => exile.chosen.len() >= exile.count,
        }
    }

    fn compute_targets(
        &mut self,
        db: &mut Database,
        source: &CardId,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> bool {
        match self {
            Cost::SacrificePermanent(sac) => {
                let valid_targets = db.battlefield[&db[source].controller]
                    .iter()
                    .filter(|target| {
                        !already_chosen.contains(&ActiveTarget::Battlefield {
                            id: (*target).clone(),
                        }) && target.passes_restrictions(
                            db,
                            LogId::current(db),
                            source,
                            &sac.restrictions,
                        )
                    })
                    .cloned()
                    .collect_vec();
                if valid_targets != sac.valid_targets {
                    sac.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            Cost::TapPermanent(tap) => {
                let valid_targets = db.battlefield[&db[source].controller]
                    .iter()
                    .filter(|target| {
                        !already_chosen.contains(&ActiveTarget::Battlefield {
                            id: (*target).clone(),
                        }) && !target.tapped(db)
                            && target.passes_restrictions(
                                db,
                                LogId::current(db),
                                source,
                                &tap.restrictions,
                            )
                    })
                    .cloned()
                    .collect_vec();
                if valid_targets != tap.valid_targets {
                    tap.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            Cost::TapPermanentsPowerXOrMore(tap) => {
                let valid_targets = db.battlefield[&db[source].controller]
                    .iter()
                    .filter(|target| {
                        !already_chosen.contains(&ActiveTarget::Battlefield {
                            id: (*target).clone(),
                        }) && !target.tapped(db)
                            && target.passes_restrictions(
                                db,
                                LogId::current(db),
                                source,
                                &tap.restrictions,
                            )
                    })
                    .cloned()
                    .collect_vec();
                if valid_targets != tap.valid_targets {
                    tap.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            Cost::SpendMana(spend) => {
                if spend.reduced {
                    return false;
                }

                if let Some(reducer) = source.faceup_face(db).cost_reducer.as_ref() {
                    match reducer.when.as_ref().unwrap() {
                        When::TargetTappedCreature(_) => {
                            if let Ok(Some(target)) = already_chosen
                                .iter()
                                .exactly_one()
                                .map(|target| target.id(db))
                            {
                                if target.tapped(db) {
                                    let reduction: ::counter::Counter<ManaCost> = reducer
                                        .reduction
                                        .iter()
                                        .map(|r| r.enum_value().unwrap())
                                        .collect();

                                    for (cost, count) in spend.paying.iter_mut() {
                                        if let Some(reduction) = reduction.get(cost) {
                                            *count = count.saturating_sub(*reduction);
                                            if *count == 0 {
                                                *count = 1;
                                            }
                                        }
                                    }

                                    spend.reduced = true;
                                    return true;
                                }
                            }
                        }
                    }
                }

                false
            }
            Cost::ExilePermanentsCmcX(exile) => {
                exile.target = already_chosen
                    .iter()
                    .map(|target| target.id(db).unwrap().faceup_face(db).cost.cmc())
                    .sum::<usize>();

                let valid_targets = db.battlefield[&db[source].controller]
                    .iter()
                    .filter(|target| {
                        target.passes_restrictions(
                            db,
                            LogId::current(db),
                            source,
                            &exile.restrictions,
                        )
                    })
                    .cloned()
                    .collect_vec();

                if valid_targets != exile.valid_targets {
                    exile.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            Cost::ExileCards(exile) => {
                let valid_targets = db
                    .cards
                    .keys()
                    .filter(|target| {
                        db[*target].controller == db[source].controller
                            && target.passes_restrictions(
                                db,
                                LogId::current(db),
                                source,
                                &exile.restrictions,
                            )
                    })
                    .cloned()
                    .collect_vec();

                if valid_targets != exile.valid_targets {
                    exile.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            Cost::ExileCardsSharingType(exile) => {
                let card_types = exile
                    .chosen
                    .iter()
                    .map(|chosen| &db[chosen].modified_types)
                    .collect_vec();

                let valid_targets = db
                    .cards
                    .keys()
                    .filter(|target| {
                        db[*target].controller == db[source].controller
                            && card_types
                                .iter()
                                .all(|types| target.types_intersect(db, types))
                    })
                    .cloned()
                    .collect_vec();

                if valid_targets != exile.valid_targets {
                    exile.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn choose_pay(
        &mut self,
        db: &mut Database,
        source_card: &CardId,
        all_targets: &HashSet<ActiveTarget>,
        choice: Option<usize>,
    ) -> bool {
        match self {
            Cost::SacrificePermanent(SacrificePermanent {
                valid_targets,
                chosen,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice].clone();
                    if !all_targets.contains(&ActiveTarget::Battlefield { id: target.clone() }) {
                        *chosen = Some(target);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Cost::TapPermanent(TapPermanent {
                valid_targets,
                chosen,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice].clone();
                    if !all_targets.contains(&ActiveTarget::Battlefield { id: target.clone() }) {
                        *chosen = Some(target);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Cost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore {
                valid_targets,
                chosen,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice].clone();
                    if !all_targets.contains(&ActiveTarget::Battlefield { id: target.clone() }) {
                        chosen.insert(target);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Cost::ExilePermanentsCmcX(ExilePermanentsCmcX {
                valid_targets,
                chosen,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice].clone();
                    if !all_targets.contains(&ActiveTarget::Battlefield { id: target.clone() }) {
                        chosen.insert(target);
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            Cost::ExileCards(ExileCards {
                valid_targets,
                chosen,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice].clone();
                    if !all_targets.contains(&target.target_from_location(db).unwrap()) {
                        chosen.insert(target);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Cost::ExileCardsSharingType(ExileCardsSharingType {
                chosen,
                valid_targets,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice].clone();
                    if !all_targets.contains(&target.target_from_location(db).unwrap()) {
                        chosen.insert(target);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Cost::SpendMana(spend) => {
                if choice.is_none() {
                    if spend
                        .paid
                        .entry(ManaCost::TWO_X)
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
                    let mut pool_post_pay = db.all_players[&db[source_card].controller]
                        .pool_post_pay(db, &mana, &source, &spend.reason)
                        .unwrap();
                    let Some(first_unpaid) = spend.first_unpaid() else {
                        return true;
                    };

                    if pool_post_pay.can_spend(db, first_unpaid, ManaSource::ANY, &spend.reason) {
                        let mana = match first_unpaid {
                            ManaCost::WHITE => Mana::WHITE,
                            ManaCost::BLUE => Mana::BLUE,
                            ManaCost::BLACK => Mana::BLACK,
                            ManaCost::RED => Mana::RED,
                            ManaCost::GREEN => Mana::GREEN,
                            ManaCost::COLORLESS => Mana::COLORLESS,
                            ManaCost::GENERIC => {
                                while matches!(spend.first_unpaid(), Some(ManaCost::GENERIC))
                                    && pool_post_pay.can_spend(
                                        db,
                                        ManaCost::GENERIC,
                                        ManaSource::ANY,
                                        &spend.reason,
                                    )
                                {
                                    let max = pool_post_pay.max(db, &spend.reason).unwrap();
                                    let (_, source) = pool_post_pay.spend(
                                        db,
                                        max,
                                        ManaSource::ANY,
                                        &spend.reason,
                                    );
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
                                    Some(ManaCost::X | ManaCost::TWO_X)
                                );
                            }
                            ManaCost::X => unreachable!(),
                            ManaCost::TWO_X => unreachable!(),
                        };
                        let (_, source) =
                            pool_post_pay.spend(db, mana, ManaSource::ANY, &spend.reason);
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
                            Some(ManaCost::X | ManaCost::TWO_X)
                        );
                    } else {
                        return false;
                    }
                }

                let (mana, sources) = spend.paying();
                if let Some((_, mana, source, _)) = db.all_players[&db[source_card].controller]
                    .pool_post_pay(db, &mana, &sources, &spend.reason)
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
                    if db.all_players[&db[source_card].controller].can_spend_mana(
                        db,
                        &mana,
                        &sources,
                        &spend.reason,
                    ) {
                        !matches!(
                            spend.first_unpaid_x_always_unpaid(),
                            Some(ManaCost::X | ManaCost::TWO_X)
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

    fn results(&self, db: &mut Database, source: &CardId) -> Vec<ActionResult> {
        match self {
            Cost::SacrificePermanent(SacrificePermanent { chosen, .. }) => {
                vec![ActionResult::PermanentToGraveyard(chosen.clone().unwrap())]
            }
            Cost::TapPermanent(TapPermanent { chosen, .. }) => {
                vec![ActionResult::TapPermanent(chosen.clone().unwrap())]
            }
            Cost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore { chosen, .. }) => chosen
                .iter()
                .map(|chosen| ActionResult::TapPermanent(chosen.clone()))
                .collect_vec(),
            Cost::ExilePermanentsCmcX(exile) => {
                let mut results = vec![];
                for target in exile.chosen.iter() {
                    results.push(ActionResult::ExileTarget {
                        source: source.clone(),
                        target: ActiveTarget::Battlefield { id: target.clone() },
                        duration: Duration::PERMANENTLY.into(),
                        reason: None,
                    });
                }
                results
            }
            Cost::SpendMana(spend) => {
                let (mana, sources) = spend.paying();
                vec![ActionResult::SpendMana {
                    card: source.clone(),
                    mana,
                    sources,
                    reason: spend.reason.clone(),
                }]
            }
            Cost::ExileCards(exile) => {
                let mut results = vec![];
                for target in exile.chosen.iter() {
                    results.push(ActionResult::ExileTarget {
                        source: source.clone(),
                        target: target.target_from_location(db).unwrap(),
                        duration: Duration::PERMANENTLY.into(),
                        reason: exile.reason,
                    });
                }

                results
            }
            Cost::ExileCardsSharingType(exile) => {
                let mut results = vec![];
                for target in exile.chosen.iter() {
                    results.push(ActionResult::ExileTarget {
                        source: source.clone(),
                        target: target.target_from_location(db).unwrap(),
                        duration: Duration::PERMANENTLY.into(),
                        reason: exile.reason,
                    });
                }

                results
            }
        }
    }

    fn x_is(&self, db: &mut Database) -> Option<usize> {
        match self {
            Cost::SacrificePermanent(_)
            | Cost::TapPermanent(_)
            | Cost::ExileCards(_)
            | Cost::ExileCardsSharingType(_) => None,
            Cost::SpendMana(spend) => spend.x_is(),
            Cost::ExilePermanentsCmcX(exile) => Some(
                exile
                    .chosen
                    .iter()
                    .map(|chosen| chosen.faceup_face(db).cost.cmc())
                    .sum::<usize>(),
            ),
            Cost::TapPermanentsPowerXOrMore(tap) => Some(
                tap.chosen
                    .iter()
                    .map(|tapped| tapped.power(db).unwrap_or_default())
                    .sum::<i32>() as usize,
            ),
        }
    }

    fn chosen_targets(&self, db: &mut Database) -> Vec<ActiveTarget> {
        match self {
            Cost::SacrificePermanent(SacrificePermanent { chosen, .. }) => chosen
                .clone()
                .map(|id| ActiveTarget::Battlefield { id })
                .into_iter()
                .collect_vec(),
            Cost::TapPermanent(TapPermanent { chosen, .. }) => chosen
                .clone()
                .map(|id| ActiveTarget::Battlefield { id })
                .into_iter()
                .collect_vec(),
            Cost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore { chosen, .. }) => chosen
                .iter()
                .map(|id| ActiveTarget::Battlefield { id: id.clone() })
                .collect_vec(),
            Cost::SpendMana(_) => vec![],
            Cost::ExilePermanentsCmcX(exile) => exile
                .chosen
                .iter()
                .map(|chosen| ActiveTarget::Battlefield { id: chosen.clone() })
                .collect_vec(),
            Cost::ExileCards(exile) => exile
                .chosen
                .iter()
                .map(|card| card.target_from_location(db).unwrap())
                .collect_vec(),
            Cost::ExileCardsSharingType(exile) => exile
                .chosen
                .iter()
                .map(|card| card.target_from_location(db).unwrap())
                .collect_vec(),
        }
    }
}

#[derive(Debug)]
pub struct PayCost {
    cost: Cost,
    pub(crate) source: CardId,
    or_else: Option<(Vec<Effect>, Vec<ActiveTarget>)>,
}

impl PayCost {
    pub(crate) fn new(source: CardId, cost: Cost) -> PayCost {
        Self {
            cost,
            source,
            or_else: None,
        }
    }

    pub(crate) fn new_or_else(
        source: CardId,
        cost: Cost,
        effects: Vec<Effect>,
        targets: Vec<ActiveTarget>,
    ) -> PayCost {
        Self {
            cost,
            source,
            or_else: Some((effects, targets)),
        }
    }

    pub(crate) fn autopay(&self, db: &Database) -> bool {
        match &self.cost {
            Cost::SacrificePermanent(_) => false,
            Cost::TapPermanent(_) => false,
            Cost::TapPermanentsPowerXOrMore(_) => false,
            Cost::ExilePermanentsCmcX(_) => false,
            Cost::SpendMana(spend) => {
                if self.or_else.is_some() {
                    return false;
                }

                debug!("Checking autopay: {:?}", spend,);
                if let Some(first_unpaid) = spend.first_unpaid_x_always_unpaid() {
                    debug!("first unpaid {:?}", first_unpaid,);
                    let (mana, source) = spend.paying();
                    match first_unpaid {
                        ManaCost::TWO_X | ManaCost::X | ManaCost::GENERIC => return false,
                        unpaid => {
                            let pool_post_pay = db.all_players[&db[&self.source].controller]
                                .pool_post_pay(db, &mana, &source, &spend.reason)
                                .unwrap();
                            if !pool_post_pay.can_spend(db, unpaid, ManaSource::ANY, &spend.reason)
                            {
                                return false;
                            }
                        }
                    }
                }

                true
            }
            Cost::ExileCards(_) => false,
            Cost::ExileCardsSharingType(_) => false,
        }
    }

    fn optional(&self) -> bool {
        match &self.cost {
            Cost::SacrificePermanent(_) => true,
            Cost::TapPermanent(_) => true,
            Cost::TapPermanentsPowerXOrMore(_) => true,
            Cost::ExilePermanentsCmcX(_) => true,
            Cost::ExileCards(ExileCards { .. }) => true,
            Cost::ExileCardsSharingType(_) => true,
            Cost::SpendMana(spend) => {
                if self.or_else.is_some() {
                    return true;
                }

                if let Some(ManaCost::TWO_X) = spend.first_unpaid_x_always_unpaid() {
                    spend
                        .paid
                        .get(&ManaCost::TWO_X)
                        .iter()
                        .flat_map(|i| i.values())
                        .flat_map(|i| i.values())
                        .sum::<usize>()
                        % 2
                        == 0
                } else {
                    true
                }
            }
        }
    }
}

impl PendingResult for PayCost {
    fn cancelable(&self, _db: &Database) -> bool {
        self.or_else.is_none() && self.optional()
    }

    fn options(&self, db: &mut Database) -> Options {
        let options =
            match &self.cost {
                Cost::SacrificePermanent(sac) => sac
                    .valid_targets
                    .iter()
                    .enumerate()
                    .map(|(idx, target)| (idx, format!("{} - ({})", target.name(db), target)))
                    .collect_vec(),
                Cost::TapPermanent(tap) => tap
                    .valid_targets
                    .iter()
                    .enumerate()
                    .map(|(idx, target)| (idx, format!("{} - ({})", target.name(db), target)))
                    .collect_vec(),
                Cost::TapPermanentsPowerXOrMore(tap) => tap
                    .valid_targets
                    .iter()
                    .enumerate()
                    .filter(|(_, chosen)| !tap.chosen.contains(*chosen))
                    .map(|(idx, target)| (idx, target.name(db).clone()))
                    .collect_vec(),
                Cost::SpendMana(spend) => {
                    let (mana, sources) = spend.paying();
                    let pool_post_paid = db.all_players[&db[&self.source].controller]
                        .pool_post_pay(db, &mana, &sources, &spend.reason);
                    if pool_post_paid.is_none()
                        || pool_post_paid
                            .as_ref()
                            .unwrap()
                            .max(db, &spend.reason)
                            .is_none()
                    {
                        return Options::OptionalList(vec![]);
                    }
                    let pool_post_paid = pool_post_paid.unwrap();

                    match spend.first_unpaid_x_always_unpaid() {
                        Some(ManaCost::GENERIC | ManaCost::X | ManaCost::TWO_X) => pool_post_paid
                            .available_pool_display()
                            .into_iter()
                            .enumerate()
                            .collect_vec(),
                        _ => vec![],
                    }
                }
                Cost::ExilePermanentsCmcX(exile) => exile
                    .valid_targets
                    .iter()
                    .enumerate()
                    .filter(|(_, chosen)| !exile.chosen.contains(*chosen))
                    .map(|(idx, target)| (idx, target.name(db).clone()))
                    .collect_vec(),
                Cost::ExileCards(exile) => {
                    if exile.chosen.len() == exile.maximum {
                        vec![]
                    } else {
                        exile
                            .valid_targets
                            .iter()
                            .enumerate()
                            .filter(|(_, chosen)| !exile.chosen.contains(*chosen))
                            .map(|(idx, target)| (idx, target.name(db).clone()))
                            .collect_vec()
                    }
                }
                Cost::ExileCardsSharingType(exile) => exile
                    .valid_targets
                    .iter()
                    .enumerate()
                    .filter(|(_, chosen)| !exile.chosen.contains(*chosen))
                    .map(|(idx, target)| (idx, target.name(db).clone()))
                    .collect_vec(),
            };

        if self.optional() {
            Options::OptionalList(options)
        } else {
            Options::MandatoryList(options)
        }
    }

    fn target_for_option(&self, _db: &Database, option: usize) -> Option<ActiveTarget> {
        match &self.cost {
            Cost::SacrificePermanent(sac) => sac
                .valid_targets
                .get(option)
                .map(|t| ActiveTarget::Battlefield { id: t.clone() }),
            Cost::TapPermanent(tap) => tap
                .valid_targets
                .get(option)
                .map(|t| ActiveTarget::Battlefield { id: t.clone() }),
            Cost::TapPermanentsPowerXOrMore(tap) => tap
                .valid_targets
                .get(option)
                .map(|t| ActiveTarget::Battlefield { id: t.clone() }),
            Cost::ExilePermanentsCmcX(exile) => exile
                .valid_targets
                .get(option)
                .map(|t| ActiveTarget::Battlefield { id: t.clone() }),
            Cost::ExileCards(exile) => {
                if exile.chosen.len() == exile.maximum {
                    None
                } else {
                    exile
                        .valid_targets
                        .get(option)
                        .map(|t| ActiveTarget::Battlefield { id: t.clone() })
                }
            }
            Cost::ExileCardsSharingType(exile) => exile
                .valid_targets
                .get(option)
                .map(|t| ActiveTarget::Battlefield { id: t.clone() }),
            Cost::SpendMana(_) => None,
        }
    }

    fn description(&self, _db: &Database) -> String {
        match &self.cost {
            Cost::SacrificePermanent(_) => "sacrificing a permanent".to_string(),
            Cost::TapPermanent(_) | Cost::TapPermanentsPowerXOrMore(_) => {
                "tapping a permanent".to_string()
            }
            Cost::SpendMana(spend) => spend.description(),
            Cost::ExilePermanentsCmcX(_) | Cost::ExileCards(_) | Cost::ExileCardsSharingType(_) => {
                "exiling a permanent".to_string()
            }
        }
    }

    fn is_empty(&self) -> bool {
        match &self.cost {
            Cost::SacrificePermanent(_) => false,
            Cost::TapPermanent(_) => false,
            Cost::TapPermanentsPowerXOrMore(_) => false,
            Cost::SpendMana(spend) => spend.is_empty(),
            Cost::ExilePermanentsCmcX(_) => false,
            Cost::ExileCards(_) => false,
            Cost::ExileCardsSharingType(_) => false,
        }
    }

    fn make_choice(
        &mut self,
        db: &mut Database,
        choice: Option<usize>,
        results: &mut PendingResults,
    ) -> bool {
        if choice.is_none() && self.or_else.is_some() {
            let (effects, targets) = self.or_else.as_ref().unwrap();
            let controller = db[&self.source].controller.clone();
            for effect in effects.iter() {
                effect.push_behavior_with_targets(
                    db,
                    targets.clone(),
                    &self.source,
                    &controller,
                    results,
                );
            }

            true
        } else if self
            .cost
            .choose_pay(db, &self.source, &results.all_chosen_targets, choice)
        {
            if self.cost.paid(db) {
                results.x_is = self.cost.x_is(db);
                debug!("X is {:?}", results.x_is);
                for target in self.cost.chosen_targets(db) {
                    results.all_chosen_targets.insert(target);
                }
                for result in self.cost.results(db, &self.source) {
                    results.push_settled(result);
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn recompute_targets(
        &mut self,
        db: &mut Database,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> bool {
        self.cost.compute_targets(db, &self.source, already_chosen)
    }
}
