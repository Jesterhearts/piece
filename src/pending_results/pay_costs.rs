use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    cost::ReduceWhen,
    effects::EffectDuration,
    in_play::{target_from_location, CardId, Database, ExileReason, OnBattlefield},
    mana::{Mana, ManaCost},
    pending_results::{PendingResult, PendingResults},
    player::{
        mana_pool::{ManaSource, SpendReason},
        AllPlayers,
    },
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug)]
pub(crate) struct SacrificePermanent {
    source: CardId,
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: Option<CardId>,
}

impl SacrificePermanent {
    pub(crate) fn new(restrictions: Vec<Restriction>, source: CardId) -> Self {
        Self {
            source,
            restrictions,
            valid_targets: Default::default(),
            chosen: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TapPermanent {
    source: CardId,
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: Option<CardId>,
}

impl TapPermanent {
    pub(crate) fn new(restrictions: Vec<Restriction>, source: CardId) -> Self {
        Self {
            source,
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
    source: CardId,
    valid_targets: Vec<CardId>,
    chosen: IndexSet<CardId>,
}

impl TapPermanentsPowerXOrMore {
    pub(crate) fn new(restrictions: Vec<Restriction>, x_is: usize, source: CardId) -> Self {
        Self {
            restrictions,
            x_is,
            source,
            valid_targets: Default::default(),
            chosen: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ExilePermanentsCmcX {
    source: CardId,
    restrictions: Vec<Restriction>,
    valid_targets: Vec<CardId>,
    chosen: IndexSet<CardId>,
    target: usize,
}

impl ExilePermanentsCmcX {
    pub(crate) fn new(restrictions: Vec<Restriction>, source: CardId) -> Self {
        Self {
            source,
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
    source: CardId,
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
        source: CardId,
    ) -> Self {
        Self {
            reason,
            source,
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
    source: CardId,
    count: usize,
    chosen: IndexSet<CardId>,
    valid_targets: Vec<CardId>,
}

impl ExileCardsSharingType {
    pub(crate) fn new(reason: Option<ExileReason>, source: CardId, count: usize) -> Self {
        Self {
            reason,
            source,
            count,
            chosen: Default::default(),
            valid_targets: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct SpendMana {
    source: CardId,
    paying: IndexMap<ManaCost, usize>,
    paid: IndexMap<ManaCost, IndexMap<Mana, IndexMap<ManaSource, usize>>>,
    reason: SpendReason,
    reduced: bool,
}

impl SpendMana {
    pub(crate) fn new(mut mana: Vec<ManaCost>, source: CardId, reason: SpendReason) -> Self {
        mana.sort();

        let mut paying = IndexMap::default();
        for cost in mana {
            *paying.entry(cost).or_default() += 1;
        }
        let mut paid = IndexMap::default();
        paid.entry(ManaCost::X).or_default();
        paid.entry(ManaCost::TwoX).or_default();

        Self {
            source,
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

    pub(crate) fn first_unpaid(&self) -> Option<ManaCost> {
        self.first_unpaid_x_always_unpaid()
            .filter(|unpaid| !matches!(unpaid, ManaCost::X | ManaCost::TwoX))
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

#[derive(Debug)]
pub(crate) enum PayCost {
    SacrificePermanent(SacrificePermanent),
    TapPermanent(TapPermanent),
    TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore),
    SpendMana(SpendMana),
    ExilePermanentsCmcX(ExilePermanentsCmcX),
    ExileCards(ExileCards),
    ExileCardsSharingType(ExileCardsSharingType),
}

impl PayCost {
    pub(crate) fn autopay(&self, db: &Database, all_players: &AllPlayers) -> bool {
        match self {
            PayCost::SacrificePermanent(_) => false,
            PayCost::TapPermanent(_) => false,
            PayCost::TapPermanentsPowerXOrMore(_) => false,
            PayCost::ExilePermanentsCmcX(_) => false,
            PayCost::SpendMana(spend) => {
                debug!("Checking autopay: {:?}", spend,);
                if let Some(first_unpaid) = spend.first_unpaid_x_always_unpaid() {
                    debug!("first unpaid {:?}", first_unpaid,);
                    let (mana, source) = spend.paying();
                    match first_unpaid {
                        ManaCost::TwoX | ManaCost::X | ManaCost::Generic(_) => return false,
                        unpaid => {
                            let pool_post_pay = all_players[spend.source.controller(db)]
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
            PayCost::ExileCards(_) => false,
            PayCost::ExileCardsSharingType(_) => false,
        }
    }

    pub(crate) fn source(&self) -> CardId {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent { source, .. })
            | PayCost::TapPermanent(TapPermanent { source, .. })
            | PayCost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore { source, .. })
            | PayCost::SpendMana(SpendMana { source, .. })
            | PayCost::ExilePermanentsCmcX(ExilePermanentsCmcX { source, .. })
            | PayCost::ExileCards(ExileCards { source, .. })
            | PayCost::ExileCardsSharingType(ExileCardsSharingType { source, .. }) => *source,
        }
    }

    fn paid(&self, db: &mut Database) -> bool {
        match self {
            PayCost::SacrificePermanent(sac) => sac.chosen.is_some(),
            PayCost::TapPermanent(tap) => tap.chosen.is_some(),
            PayCost::TapPermanentsPowerXOrMore(tap) => {
                tap.chosen
                    .iter()
                    .map(|card| card.power(db).unwrap_or_default())
                    .sum::<i32>()
                    >= tap.x_is as i32
            }
            PayCost::SpendMana(spend) => spend.paid(),
            PayCost::ExilePermanentsCmcX(exile) => {
                exile
                    .chosen
                    .iter()
                    .map(|chosen| chosen.cost(db).cmc())
                    .sum::<usize>()
                    >= exile.target
            }
            PayCost::ExileCards(exile) => exile.chosen.len() >= exile.minimum,
            PayCost::ExileCardsSharingType(exile) => exile.chosen.len() >= exile.count,
        }
    }

    fn compute_targets(
        &mut self,
        db: &mut Database,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> bool {
        match self {
            PayCost::SacrificePermanent(sac) => {
                let controller = sac.source.controller(db);
                let valid_targets = controller
                    .get_cards_in::<OnBattlefield>(db)
                    .into_iter()
                    .filter(|target| {
                        !already_chosen.contains(&ActiveTarget::Battlefield { id: *target })
                            && target.passes_restrictions(db, sac.source, &sac.restrictions)
                    })
                    .collect_vec();
                if valid_targets != sac.valid_targets {
                    sac.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            PayCost::TapPermanent(tap) => {
                let controller = tap.source.controller(db);
                let valid_targets = controller
                    .get_cards_in::<OnBattlefield>(db)
                    .into_iter()
                    .filter(|target| {
                        !already_chosen.contains(&ActiveTarget::Battlefield { id: *target })
                            && !target.tapped(db)
                            && target.passes_restrictions(db, tap.source, &tap.restrictions)
                    })
                    .collect_vec();
                if valid_targets != tap.valid_targets {
                    tap.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            PayCost::TapPermanentsPowerXOrMore(tap) => {
                let controller = tap.source.controller(db);
                let valid_targets = controller
                    .get_cards_in::<OnBattlefield>(db)
                    .into_iter()
                    .filter(|target| {
                        !already_chosen.contains(&ActiveTarget::Battlefield { id: *target })
                            && !target.tapped(db)
                            && target.passes_restrictions(db, tap.source, &tap.restrictions)
                    })
                    .collect_vec();
                if valid_targets != tap.valid_targets {
                    tap.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            PayCost::SpendMana(spend) => {
                if spend.reduced {
                    return false;
                }

                if let Some(reducer) = spend.source.cost_reducer(db) {
                    match reducer.when {
                        ReduceWhen::TargetTappedCreature => {
                            if let Ok(Some(target)) = already_chosen
                                .iter()
                                .exactly_one()
                                .map(|target| target.id())
                            {
                                if target.tapped(db) {
                                    let (reducing, count) = spend
                                        .paying
                                        .iter()
                                        .find(|paying| {
                                            std::mem::discriminant(paying.0)
                                                == std::mem::discriminant(&reducer.reduction)
                                        })
                                        .map(|(cost, count)| (*cost, *count))
                                        .unwrap();

                                    spend.paying.remove(&reducing);

                                    match reducing {
                                        ManaCost::Generic(count) => {
                                            let ManaCost::Generic(reduce) = reducer.reduction
                                            else {
                                                unreachable!()
                                            };

                                            if reduce < count {
                                                spend
                                                    .paying
                                                    .insert(ManaCost::Generic(count - reduce), 1);
                                            }
                                        }
                                        _ => {
                                            spend.paying.insert(reducing, count - 1);
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
            PayCost::ExilePermanentsCmcX(exile) => {
                exile.target = already_chosen
                    .iter()
                    .map(|target| target.id().unwrap().cost(db).cmc())
                    .sum::<usize>();

                let controller = exile.source.controller(db);
                let valid_targets = controller
                    .get_cards_in::<OnBattlefield>(db)
                    .into_iter()
                    .filter(|target| {
                        target.passes_restrictions(db, exile.source, &exile.restrictions)
                    })
                    .collect_vec();

                if valid_targets != exile.valid_targets {
                    exile.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            PayCost::ExileCards(exile) => {
                let controller = exile.source.controller(db);
                let valid_targets = controller
                    .get_cards(db)
                    .into_iter()
                    .filter(|target| {
                        target.passes_restrictions(db, exile.source, &exile.restrictions)
                    })
                    .collect_vec();

                if valid_targets != exile.valid_targets {
                    exile.valid_targets = valid_targets;
                    true
                } else {
                    false
                }
            }
            PayCost::ExileCardsSharingType(exile) => {
                let controller = exile.source.controller(db);
                let card_types = exile
                    .chosen
                    .iter()
                    .map(|chosen| chosen.types(db))
                    .collect_vec();

                let valid_targets = controller
                    .get_cards(db)
                    .into_iter()
                    .filter(|target| {
                        card_types
                            .iter()
                            .all(|types| target.types_intersect(db, types))
                    })
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
        all_players: &mut AllPlayers,
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
            PayCost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore {
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
            PayCost::ExileCards(ExileCards {
                valid_targets,
                chosen,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice];
                    if !all_targets.contains(&target_from_location(db, target)) {
                        chosen.insert(target);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            PayCost::ExileCardsSharingType(ExileCardsSharingType {
                chosen,
                valid_targets,
                ..
            }) => {
                if let Some(choice) = choice {
                    let target = valid_targets[choice];
                    if !all_targets.contains(&target_from_location(db, target)) {
                        chosen.insert(target);
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
                    let mut pool_post_pay = all_players[spend.source.controller(db)]
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
                if let Some((_, mana, source, _)) = all_players[spend.source.controller(db)]
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
                    if all_players[spend.source.controller(db)].can_spend_mana(
                        db,
                        &mana,
                        &sources,
                        spend.reason,
                    ) {
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

    fn results(&self, db: &mut Database) -> Vec<ActionResult> {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent { chosen, .. }) => {
                vec![ActionResult::PermanentToGraveyard(chosen.unwrap())]
            }
            PayCost::TapPermanent(TapPermanent { chosen, .. }) => {
                vec![ActionResult::TapPermanent(chosen.unwrap())]
            }
            PayCost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore { chosen, .. }) => chosen
                .iter()
                .map(|chosen| ActionResult::TapPermanent(*chosen))
                .collect_vec(),
            PayCost::ExilePermanentsCmcX(exile) => {
                let mut results = vec![];
                for target in exile.chosen.iter() {
                    results.push(ActionResult::ExileTarget {
                        source: exile.source,
                        target: ActiveTarget::Battlefield { id: *target },
                        duration: EffectDuration::Permanently,
                        reason: None,
                    });
                }
                results
            }
            PayCost::SpendMana(spend) => {
                let (mana, sources) = spend.paying();
                vec![ActionResult::SpendMana {
                    card: spend.source,
                    mana,
                    sources,
                    reason: spend.reason,
                }]
            }
            PayCost::ExileCards(exile) => {
                let mut results = vec![];
                for target in exile.chosen.iter() {
                    results.push(ActionResult::ExileTarget {
                        source: exile.source,
                        target: target_from_location(db, *target),
                        duration: EffectDuration::Permanently,
                        reason: exile.reason,
                    });
                }

                results
            }
            PayCost::ExileCardsSharingType(exile) => {
                let mut results = vec![];
                for target in exile.chosen.iter() {
                    results.push(ActionResult::ExileTarget {
                        source: exile.source,
                        target: target_from_location(db, *target),
                        duration: EffectDuration::Permanently,
                        reason: exile.reason,
                    });
                }

                results
            }
        }
    }

    fn x_is(&self, db: &mut Database) -> Option<usize> {
        match self {
            PayCost::SacrificePermanent(_)
            | PayCost::TapPermanent(_)
            | PayCost::ExileCards(_)
            | PayCost::ExileCardsSharingType(_) => None,
            PayCost::SpendMana(spend) => spend.x_is(),
            PayCost::ExilePermanentsCmcX(exile) => Some(
                exile
                    .chosen
                    .iter()
                    .map(|chosen| chosen.cost(db).cmc())
                    .sum::<usize>(),
            ),
            PayCost::TapPermanentsPowerXOrMore(tap) => Some(
                tap.chosen
                    .iter()
                    .map(|tapped| tapped.power(db).unwrap_or_default())
                    .sum::<i32>() as usize,
            ),
        }
    }

    fn chosen_targets(&self, db: &mut Database) -> Vec<ActiveTarget> {
        match self {
            PayCost::SacrificePermanent(SacrificePermanent { chosen, .. }) => chosen
                .map(|id| ActiveTarget::Battlefield { id })
                .into_iter()
                .collect_vec(),
            PayCost::TapPermanent(TapPermanent { chosen, .. }) => chosen
                .map(|id| ActiveTarget::Battlefield { id })
                .into_iter()
                .collect_vec(),
            PayCost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore { chosen, .. }) => chosen
                .iter()
                .map(|id| ActiveTarget::Battlefield { id: *id })
                .collect_vec(),
            PayCost::SpendMana(_) => vec![],
            PayCost::ExilePermanentsCmcX(exile) => exile
                .chosen
                .iter()
                .map(|chosen| ActiveTarget::Battlefield { id: *chosen })
                .collect_vec(),
            PayCost::ExileCards(exile) => exile
                .chosen
                .iter()
                .map(|card| target_from_location(db, *card))
                .collect_vec(),
            PayCost::ExileCardsSharingType(exile) => exile
                .chosen
                .iter()
                .map(|card| target_from_location(db, *card))
                .collect_vec(),
        }
    }
}

impl PendingResult for PayCost {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        match self {
            PayCost::SacrificePermanent(_) => true,
            PayCost::TapPermanent(_) => true,
            PayCost::TapPermanentsPowerXOrMore(_) => true,
            PayCost::ExilePermanentsCmcX(_) => true,
            PayCost::ExileCards(ExileCards { .. }) => true,
            PayCost::ExileCardsSharingType(_) => true,
            PayCost::SpendMana(spend) => {
                if let Some(ManaCost::TwoX) = spend.first_unpaid_x_always_unpaid() {
                    spend
                        .paid
                        .get(&ManaCost::TwoX)
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

    fn options(&self, db: &mut Database, all_players: &AllPlayers) -> Vec<(usize, String)> {
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
            PayCost::TapPermanentsPowerXOrMore(tap) => tap
                .valid_targets
                .iter()
                .enumerate()
                .filter(|(_, chosen)| !tap.chosen.contains(*chosen))
                .map(|(idx, target)| (idx, target.name(db)))
                .collect_vec(),
            PayCost::SpendMana(spend) => {
                let (mana, sources) = spend.paying();
                let pool_post_paid = all_players[spend.source.controller(db)].pool_post_pay(
                    db,
                    &mana,
                    &sources,
                    spend.reason,
                );
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
            PayCost::ExileCards(exile) => {
                if exile.chosen.len() == exile.maximum {
                    vec![]
                } else {
                    exile
                        .valid_targets
                        .iter()
                        .enumerate()
                        .filter(|(_, chosen)| !exile.chosen.contains(*chosen))
                        .map(|(idx, target)| (idx, target.name(db)))
                        .collect_vec()
                }
            }
            PayCost::ExileCardsSharingType(exile) => exile
                .valid_targets
                .iter()
                .enumerate()
                .filter(|(_, chosen)| !exile.chosen.contains(*chosen))
                .map(|(idx, target)| (idx, target.name(db)))
                .collect_vec(),
        }
    }

    fn description(&self, _db: &Database) -> String {
        match self {
            PayCost::SacrificePermanent(_) => "sacrificing a permanent".to_string(),
            PayCost::TapPermanent(_) | PayCost::TapPermanentsPowerXOrMore(_) => {
                "tapping a permanent".to_string()
            }
            PayCost::SpendMana(spend) => spend.description(),
            PayCost::ExilePermanentsCmcX(_)
            | PayCost::ExileCards(_)
            | PayCost::ExileCardsSharingType(_) => "exiling a permanent".to_string(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            PayCost::SacrificePermanent(_) => false,
            PayCost::TapPermanent(_) => false,
            PayCost::TapPermanentsPowerXOrMore(_) => false,
            PayCost::SpendMana(spend) => spend.is_empty(),
            PayCost::ExilePermanentsCmcX(_) => false,
            PayCost::ExileCards(_) => false,
            PayCost::ExileCardsSharingType(_) => false,
        }
    }

    fn make_choice(
        &mut self,
        db: &mut Database,
        all_players: &mut AllPlayers,
        choice: Option<usize>,
        results: &mut PendingResults,
    ) -> bool {
        if self.choose_pay(db, all_players, &results.all_chosen_targets, choice) {
            if self.paid(db) {
                results.x_is = self.x_is(db);
                debug!("X is {:?}", results.x_is);
                for target in self.chosen_targets(db) {
                    results.all_chosen_targets.insert(target);
                }
                for result in self.results(db) {
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
        self.compute_targets(db, already_chosen)
    }
}
