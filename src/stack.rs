use std::{
    collections::HashSet,
    sync::atomic::{AtomicUsize, Ordering},
};

use derive_more::{Deref, DerefMut};
use indexmap::IndexMap;
use itertools::Itertools;
use tracing::Level;

use crate::{
    abilities::{Ability, TriggeredAbility},
    battlefield::{ActionResult, Battlefields},
    card::{Color, Keyword},
    cost::AdditionalCost,
    effects::EffectBehaviors,
    in_play::{CardId, CastFrom, Database},
    log::{Log, LogEntry, LogId},
    pending_results::{
        choose_targets::ChooseTargets,
        pay_costs::TapPermanent,
        pay_costs::{Cost, ExileCardsSharingType},
        pay_costs::{ExileCards, ExilePermanentsCmcX},
        pay_costs::{PayCost, SpendMana},
        pay_costs::{SacrificePermanent, TapPermanentsPowerXOrMore},
        PendingResults, Source, TargetSource,
    },
    player::{mana_pool::SpendReason, Owner},
    targets::{Cmc, Comparison, ControllerRestriction, Dynamic, Location, Restriction},
    triggers::TriggerSource,
};

static NEXT_STACK_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct StackId(usize);

impl StackId {
    pub(crate) fn new() -> Self {
        Self(NEXT_STACK_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl std::fmt::Display for StackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Debug)]
enum ResolutionType {
    Card(CardId),
    Ability(CardId),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub(crate) enum ActiveTarget {
    Stack { id: StackId },
    Battlefield { id: CardId },
    Graveyard { id: CardId },
    Library { id: CardId },
    Player { id: Owner },
}

impl ActiveTarget {
    pub(crate) fn display(&self, db: &Database) -> String {
        match self {
            ActiveTarget::Stack { id } => {
                format!(
                    "Stack ({}): {}",
                    id,
                    db.stack.entries.get(id).unwrap().display(db)
                )
            }
            ActiveTarget::Battlefield { id } => {
                format!("{} - ({})", id.name(db), id)
            }
            ActiveTarget::Graveyard { id } => {
                format!("{} - ({})", id.name(db), id)
            }
            ActiveTarget::Library { id } => {
                format!("{} - ({})", id.name(db), id)
            }
            ActiveTarget::Player { id } => db.all_players[*id].name.clone(),
        }
    }

    pub(crate) fn id(&self) -> Option<CardId> {
        match self {
            ActiveTarget::Battlefield { id }
            | ActiveTarget::Graveyard { id }
            | ActiveTarget::Library { id } => Some(*id),
            ActiveTarget::Stack { .. } => None,
            ActiveTarget::Player { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Entry {
    Card(CardId),
    Ability { source: CardId, ability: Ability },
}

impl Entry {
    pub(crate) fn source(&self) -> CardId {
        match self {
            Entry::Card(source) | Entry::Ability { source, .. } => *source,
        }
    }
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub(crate) struct Modes(pub(crate) Vec<usize>);

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub(crate) targets: Vec<Vec<ActiveTarget>>,
    pub(crate) ty: Entry,
    pub(crate) mode: Vec<usize>,
    pub(crate) settled: bool,
}

impl StackEntry {
    pub fn display(&self, db: &Database) -> String {
        match &self.ty {
            Entry::Card(card) => card.faceup_face(db).name.clone(),
            Entry::Ability {
                source: card_source,
                ability,
            } => {
                format!("{}: {}", db[*card_source].modified_name, ability.text(db))
            }
        }
    }

    pub(crate) fn passes_restrictions(
        &self,
        db: &Database,
        log_session: LogId,
        source: CardId,
        restrictions: &[Restriction],
    ) -> bool {
        let spell_or_ability_controller = db[self.ty.source()].controller;

        for restriction in restrictions.iter() {
            match restriction {
                Restriction::AttackedThisTurn => todo!(),
                Restriction::Attacking => unreachable!(),
                Restriction::AttackingOrBlocking => unreachable!(),
                Restriction::CastFromHand => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if !matches!(db[*card].cast_from, Some(CastFrom::Hand)) {
                        return false;
                    }
                }
                Restriction::Cmc(cmc_test) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };
                    let cmc = db[*card].modified_cost.cmc() as i32;

                    match cmc_test {
                        Cmc::Comparison(comparison) => {
                            let matches = match comparison {
                                Comparison::LessThan(i) => cmc < *i,
                                Comparison::LessThanOrEqual(i) => cmc <= *i,
                                Comparison::GreaterThan(i) => cmc > *i,
                                Comparison::GreaterThanOrEqual(i) => cmc >= *i,
                            };
                            if !matches {
                                return false;
                            }
                        }
                        Cmc::Dynamic(dy) => match dy {
                            Dynamic::X => {
                                if source.get_x(db) as i32 != cmc {
                                    return false;
                                }
                            }
                        },
                    }
                }
                Restriction::Controller(controller_restriction) => {
                    match controller_restriction {
                        ControllerRestriction::Self_ => {
                            if db[source].controller != spell_or_ability_controller {
                                return false;
                            }
                        }
                        ControllerRestriction::Opponent => {
                            if db[source].controller == spell_or_ability_controller {
                                return false;
                            }
                        }
                    };
                }
                Restriction::ControllerControlsBlackOrGreen => {
                    let colors = Battlefields::controlled_colors(db, spell_or_ability_controller);
                    if !(colors.contains(&Color::Green) || colors.contains(&Color::Black)) {
                        return false;
                    }
                }
                Restriction::ControllerHandEmpty => {
                    if spell_or_ability_controller.has_cards(db, Location::Hand) {
                        return false;
                    }
                }
                Restriction::ControllerJustCast => {
                    if !Log::session(db, log_session).iter().any(|(_, entry)| {
                        let LogEntry::Cast { card } = entry else {
                            return false;
                        };
                        db[*card].controller == spell_or_ability_controller
                    }) {
                        return false;
                    }
                }
                Restriction::Descend(count) => {
                    let cards = db.graveyard[spell_or_ability_controller]
                        .iter()
                        .filter(|card| card.is_permanent(db))
                        .count();
                    if cards < *count {
                        return false;
                    }
                }
                Restriction::DescendedThisTurn => {
                    let descended = db
                        .graveyard
                        .descended_this_turn
                        .get(&Owner::from(spell_or_ability_controller))
                        .copied()
                        .unwrap_or_default();
                    if descended < 1 {
                        return false;
                    }
                }
                Restriction::DuringControllersTurn => {
                    if spell_or_ability_controller != db.turn.active_player() {
                        return false;
                    }
                }
                Restriction::EnteredTheBattlefieldThisTurn {
                    count,
                    restrictions,
                } => {
                    let entered_this_turn = CardId::entered_battlefield_this_turn(db)
                        .filter(|card| {
                            card.passes_restrictions(db, log_session, source, restrictions)
                        })
                        .count();
                    if entered_this_turn < *count {
                        return false;
                    }
                }
                Restriction::HasActivatedAbility => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if db[*card].modified_activated_abilities.is_empty() {
                        return false;
                    }
                }
                Restriction::InGraveyard => {
                    return false;
                }
                Restriction::InLocation { locations } => {
                    if locations
                        .iter()
                        .any(|location| !matches!(location, Location::Stack))
                    {
                        return false;
                    }
                }
                Restriction::LifeGainedThisTurn(count) => {
                    let gained_this_turn = db
                        .turn
                        .life_gained_this_turn
                        .get(&Owner::from(spell_or_ability_controller))
                        .copied()
                        .unwrap_or_default();
                    if gained_this_turn < *count {
                        return false;
                    }
                }
                Restriction::ManaSpentFromSource(source) => {
                    let Entry::Card(card) = &self.ty else {
                        // TODO: Pretty sure there are some mana sources that copy abilities if used.
                        return false;
                    };

                    if !db[*card].sourced_mana.contains_key(source) {
                        return false;
                    }
                }
                Restriction::NonToken => {
                    if !matches!(self.ty, Entry::Card(_)) {
                        return false;
                    }
                }
                Restriction::NotChosen => {
                    let Entry::Card(candidate) = &self.ty else {
                        return false;
                    };

                    if Log::session(db, log_session).iter().any(|(_, entry)| {
                        let LogEntry::CardChosen { card } = entry else {
                            return false;
                        };
                        *card == *candidate
                    }) {
                        return false;
                    }
                }
                Restriction::NotKeywords(not_keywords) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if db[*card]
                        .modified_keywords
                        .keys()
                        .any(|keyword| not_keywords.contains_key(keyword))
                    {
                        return false;
                    }
                }
                Restriction::NotOfType { types, subtypes } => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if !types.is_empty()
                        && db[*card]
                            .modified_types
                            .iter()
                            .any(|ty| types.contains_key(ty))
                    {
                        return false;
                    }
                    if !subtypes.is_empty()
                        && db[*card]
                            .modified_subtypes
                            .iter()
                            .any(|ty| subtypes.contains_key(ty))
                    {
                        return false;
                    }
                }
                Restriction::NotSelf => {
                    let Entry::Card(card) = &self.ty else {
                        continue;
                    };

                    if source == *card {
                        return false;
                    }
                }
                Restriction::NumberOfCountersOnThis { .. } => {
                    return false;
                }
                Restriction::OfColor(of_colors) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if db[*card].modified_colors.is_disjoint(of_colors) {
                        return false;
                    }
                }
                Restriction::OfType { types, subtypes } => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if !types.is_empty()
                        && !db[*card]
                            .modified_types
                            .iter()
                            .any(|ty| types.contains_key(ty))
                    {
                        return false;
                    }
                    if !subtypes.is_empty()
                        && !db[*card]
                            .modified_subtypes
                            .iter()
                            .any(|ty| subtypes.contains_key(ty))
                    {
                        return false;
                    }
                }
                Restriction::OnBattlefield => {
                    return false;
                }
                Restriction::Power(comparison) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if let Some(power) = card.power(db) {
                        if !match comparison {
                            Comparison::LessThan(target) => power < *target,
                            Comparison::LessThanOrEqual(target) => power <= *target,
                            Comparison::GreaterThan(target) => power > *target,
                            Comparison::GreaterThanOrEqual(target) => power >= *target,
                        } {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                Restriction::Self_ => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if *card != source {
                        return false;
                    }
                }
                Restriction::SourceCast => {
                    if db[source].cast_from.is_none() {
                        return false;
                    }
                }
                Restriction::SpellOrAbilityJustCast => {
                    match &self.ty {
                        Entry::Card(candidate) => {
                            if !Log::session(db, log_session).iter().any(|(_, entry)| {
                                if let LogEntry::Cast { card } = entry {
                                    *card == *candidate
                                } else {
                                    false
                                }
                            }) {
                                return false;
                            }
                        }
                        Entry::Ability {
                            source,
                            ability: Ability::Activated(candidate),
                        } => {
                            if !Log::session(db, log_session).iter().any(|(_, entry)| {
                                if let LogEntry::Activated { card, ability } = entry {
                                    event!(Level::DEBUG, ?card, ?source, ?ability, ?candidate);
                                    *card == *source && *ability == *candidate
                                } else {
                                    false
                                }
                            }) {
                                return false;
                            }
                        }
                        _ => {
                            return false;
                        }
                    };
                }
                Restriction::Tapped => {
                    return false;
                }
                Restriction::TargetedBy => {
                    if !Log::session(db, log_session).iter().any(|(_, entry)| {
                        if let LogEntry::Targeted {
                            source: targeting,
                            target,
                        } = entry
                        {
                            self.ty.source() == *targeting && *target == source
                        } else {
                            false
                        }
                    }) {
                        return false;
                    }
                }
                Restriction::Threshold => {
                    if db.graveyard[spell_or_ability_controller].len() < 7 {
                        return false;
                    }
                }
                Restriction::Toughness(comparison) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if let Some(toughness) = card.toughness(db) {
                        if !match comparison {
                            Comparison::LessThan(target) => toughness < *target,
                            Comparison::LessThanOrEqual(target) => toughness <= *target,
                            Comparison::GreaterThan(target) => toughness > *target,
                            Comparison::GreaterThanOrEqual(target) => toughness >= *target,
                        } {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
        }

        true
    }
}

#[derive(Debug, Default)]
pub struct Stack {
    pub(crate) entries: IndexMap<StackId, StackEntry>,
}

impl Stack {
    pub(crate) fn contains(&self, card: CardId) -> bool {
        self.entries
            .iter()
            .any(|(_, entry)| matches!(entry.ty, Entry::Card(entry) if entry == card))
    }

    pub(crate) fn split_second(&self, db: &Database) -> bool {
        if let Some((
            _,
            StackEntry {
                ty: Entry::Card(card),
                ..
            },
        )) = self.entries.last()
        {
            db[*card]
                .modified_keywords
                .contains_key(Keyword::SplitSecond.as_ref())
        } else {
            false
        }
    }

    pub(crate) fn remove(&mut self, card: CardId) {
        self.entries
            .retain(|_, entry| !matches!(entry.ty, Entry::Card(entry) if entry == card));
    }

    #[cfg(test)]
    pub(crate) fn target_nth(&self, nth: usize) -> ActiveTarget {
        let id = self.entries.get_index(nth).unwrap().0;
        ActiveTarget::Stack { id: *id }
    }

    pub fn entries(&self) -> Vec<StackEntry> {
        self.entries.values().cloned().collect_vec()
    }

    pub fn entries_unsettled(&self) -> Vec<StackEntry> {
        self.entries
            .values()
            .filter(|entry| !entry.settled)
            .cloned()
            .collect_vec()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn settle(&mut self) {
        for entry in self.entries.values_mut() {
            entry.settled = true;
        }
    }

    pub fn resolve_1(db: &mut Database) -> PendingResults {
        let Some((_, next)) = db.stack.entries.pop() else {
            return PendingResults::default();
        };

        db.stack.settle();

        let (apply_to_self, effects, controller, resolving_card, source, ty) = match next.ty {
            Entry::Card(card) => {
                let effects = if !card.faceup_face(db).modes.is_empty() {
                    debug!("Modes: {:?}", card.faceup_face(db).modes);
                    card.faceup_face(db).modes[next.mode.into_iter().exactly_one().unwrap()]
                        .effects
                        .clone()
                } else {
                    card.faceup_face(db).effects.clone()
                };

                (
                    false,
                    effects,
                    db[card].controller,
                    Some(card),
                    card,
                    ResolutionType::Card(card),
                )
            }
            Entry::Ability { source, ability } => (
                ability.apply_to_self(db),
                ability.effects(db),
                db[source].controller,
                None,
                source,
                ResolutionType::Ability(source),
            ),
        };

        let mut results = PendingResults::default();
        results.apply_in_stages();

        let mut targets = next.targets.into_iter();
        for (effect, targets) in effects
            .into_iter()
            .zip((&mut targets).chain(std::iter::repeat(vec![])))
        {
            let effect = effect.effect;
            if targets.len() != effect.needs_targets(db, source)
                && effect.needs_targets(db, source) != 0
            {
                let valid_targets = effect.valid_targets(
                    db,
                    source,
                    crate::log::LogId::current(db),
                    controller,
                    &HashSet::default(),
                );
                results.push_choose_targets(ChooseTargets::new(
                    TargetSource::Effect(effect),
                    valid_targets,
                    crate::log::LogId::current(db),
                    source,
                ));
                continue;
            }

            if effect.wants_targets(db, source) > 0 {
                let valid_targets = effect
                    .valid_targets(
                        db,
                        source,
                        crate::log::LogId::current(db),
                        controller,
                        &HashSet::default(),
                    )
                    .into_iter()
                    .collect::<HashSet<_>>();
                if !targets.iter().all(|target| valid_targets.contains(target)) {
                    warn!(
                        "Did not match targets: {:?} vs valid {:?}",
                        targets, valid_targets
                    );
                    if let Some(resolving_card) = resolving_card {
                        let mut results = PendingResults::default();
                        results.push_settled(ActionResult::StackToGraveyard(resolving_card));
                        return results;
                    } else {
                        return PendingResults::default();
                    }
                }
            }

            effect.push_behavior_with_targets(
                db,
                targets,
                apply_to_self,
                source,
                controller,
                &mut results,
            );
        }

        if let Some(resolving_card) = resolving_card {
            if resolving_card.is_permanent(db) {
                results.push_settled(ActionResult::AddToBattlefield(
                    resolving_card,
                    targets.next().and_then(|targets| {
                        targets.into_iter().find_map(|target| match target {
                            ActiveTarget::Battlefield { id } => Some(id),
                            _ => None,
                        })
                    }),
                ));
            } else {
                results.push_settled(ActionResult::StackToGraveyard(resolving_card));
            }
        }

        match ty {
            ResolutionType::Card(card) => Log::spell_resolved(db, card),
            ResolutionType::Ability(source) => Log::ability_resolved(db, source),
        }

        results
    }

    pub(crate) fn move_etb_ability_to_stack(
        db: &mut Database,
        ability: Ability,
        source: CardId,
    ) -> PendingResults {
        let mut results = PendingResults::default();

        let targets = source.targets_for_ability(db, &ability, &HashSet::default());
        results.push_settled(ActionResult::AddAbilityToStack {
            ability,
            source,
            targets,
            x_is: None,
        });

        results
    }

    pub(crate) fn move_trigger_to_stack(
        db: &mut Database,
        listener: CardId,
        trigger: TriggeredAbility,
    ) -> PendingResults {
        let mut results = PendingResults::default();

        let mut targets = vec![];
        let controller = db[listener].controller;
        for effect in trigger.effects.iter() {
            targets.push(effect.effect.valid_targets(
                db,
                listener,
                crate::log::LogId::current(db),
                controller,
                &HashSet::default(),
            ));
        }

        results.push_settled(ActionResult::AddTriggerToStack {
            source: listener,
            trigger,
            targets,
        });

        results
    }

    pub(crate) fn move_card_to_stack_from_hand(
        db: &mut Database,
        card: CardId,
        paying_costs: bool,
    ) -> PendingResults {
        add_card_to_stack(db, card, Some(CastFrom::Hand), paying_costs)
    }

    pub(crate) fn move_card_to_stack_from_exile(
        db: &mut Database,
        card: CardId,
        paying_costs: bool,
    ) -> PendingResults {
        add_card_to_stack(db, card, Some(CastFrom::Exile), paying_costs)
    }

    pub(crate) fn push_card(
        db: &mut Database,
        source: CardId,
        targets: Vec<Vec<ActiveTarget>>,
        chosen_modes: Vec<usize>,
    ) -> PendingResults {
        db.stack.entries.insert(
            StackId::new(),
            StackEntry {
                ty: Entry::Card(source),
                targets: targets.clone(),
                settled: true,
                mode: chosen_modes,
            },
        );

        let mut results = PendingResults::default();
        for target in targets.into_iter().flat_map(|t| t.into_iter()) {
            if let ActiveTarget::Battlefield { id } = target {
                Log::targetted(db, source, id);
                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::Targeted) {
                    if listener == id
                        && source.passes_restrictions(
                            db,
                            LogId::current(db),
                            listener,
                            &trigger.trigger.restrictions,
                        )
                    {
                        results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                    }
                }
            }
        }

        results
    }

    pub(crate) fn push_ability(
        db: &mut Database,
        source: CardId,
        ability: Ability,
        targets: Vec<Vec<ActiveTarget>>,
    ) -> PendingResults {
        db.stack.entries.insert(
            StackId::new(),
            StackEntry {
                ty: Entry::Ability { source, ability },
                targets: targets.clone(),
                mode: vec![],
                settled: true,
            },
        );

        let mut results = PendingResults::default();
        for target in targets.into_iter().flat_map(|t| t.into_iter()) {
            if let ActiveTarget::Battlefield { id } = target {
                Log::targetted(db, source, id);
                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::Targeted) {
                    if listener == id
                        && source.passes_restrictions(
                            db,
                            LogId::current(db),
                            listener,
                            &trigger.trigger.restrictions,
                        )
                    {
                        results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                    }
                }
            }
        }

        results
    }
}

pub(crate) fn add_card_to_stack(
    db: &mut Database,
    card: CardId,
    from: Option<CastFrom>,
    paying_costs: bool,
) -> PendingResults {
    let mut results = PendingResults::default();

    db[card].cast_from = from;
    card.apply_modifiers_layered(db);

    if card.has_modes(db) {
        results.push_choose_mode(Source::Card(card));
    }

    results.add_card_to_stack(card, from);
    if card.wants_targets(db).into_iter().sum::<usize>() > 0 {
        let controller = db[card].controller;
        if card.faceup_face(db).enchant.is_some() {
            results.push_choose_targets(ChooseTargets::new(
                TargetSource::Aura(card),
                card.targets_for_aura(db).unwrap(),
                crate::log::LogId::current(db),
                card,
            ))
        }

        if card.faceup_face(db).effects.len() == 1 {
            let effect = &card
                .faceup_face(db)
                .effects
                .iter()
                .exactly_one()
                .unwrap()
                .effect;
            let valid_targets = effect.valid_targets(
                db,
                card,
                crate::log::LogId::current(db),
                controller,
                &HashSet::default(),
            );
            if valid_targets.len() < effect.needs_targets(db, card) {
                return PendingResults::default();
            }

            results.push_choose_targets(ChooseTargets::new(
                TargetSource::Effect(effect.clone()),
                valid_targets,
                crate::log::LogId::current(db),
                card,
            ));
        } else {
            for effect in card.faceup_face(db).effects.iter() {
                let valid_targets = effect.effect.valid_targets(
                    db,
                    card,
                    crate::log::LogId::current(db),
                    controller,
                    &HashSet::default(),
                );
                results.push_choose_targets(ChooseTargets::new(
                    TargetSource::Effect(effect.effect.clone()),
                    valid_targets,
                    crate::log::LogId::current(db),
                    card,
                ));
            }
        }
    }

    // It is important that paying costs happens last, because some cards have effects that depend on what they are targeting.
    let cost = &card.faceup_face(db).cost;
    if paying_costs {
        results.push_pay_costs(PayCost::new(
            card,
            Cost::SpendMana(SpendMana::new(
                cost.mana_cost.clone(),
                SpendReason::Casting(card),
            )),
        ));
    }
    for cost in cost.additional_cost.iter() {
        match cost {
            AdditionalCost::DiscardThis => unreachable!(),
            AdditionalCost::SacrificeSource => unreachable!(),
            AdditionalCost::RemoveCounter { .. } => unreachable!(),
            AdditionalCost::PayLife(_) => todo!(),
            AdditionalCost::SacrificePermanent(restrictions) => {
                results.push_pay_costs(PayCost::new(
                    card,
                    Cost::SacrificePermanent(SacrificePermanent::new(restrictions.clone())),
                ));
            }
            AdditionalCost::TapPermanent(restrictions) => {
                results.push_pay_costs(PayCost::new(
                    card,
                    Cost::TapPermanent(TapPermanent::new(restrictions.clone())),
                ));
            }
            AdditionalCost::TapPermanentsPowerXOrMore { x_is, restrictions } => {
                results.push_pay_costs(PayCost::new(
                    card,
                    Cost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore::new(
                        restrictions.clone(),
                        *x_is,
                    )),
                ));
            }
            AdditionalCost::ExileCardsCmcX(restrictions) => {
                results.push_pay_costs(PayCost::new(
                    card,
                    Cost::ExilePermanentsCmcX(ExilePermanentsCmcX::new(restrictions.clone())),
                ));
            }
            AdditionalCost::ExileCard { restrictions } => {
                results.push_pay_costs(PayCost::new(
                    card,
                    Cost::ExileCards(ExileCards::new(None, 1, 1, restrictions.clone())),
                ));
            }
            AdditionalCost::ExileXOrMoreCards {
                minimum,
                restrictions,
            } => {
                results.push_pay_costs(PayCost::new(
                    card,
                    Cost::ExileCards(ExileCards::new(
                        None,
                        *minimum,
                        usize::MAX,
                        restrictions.clone(),
                    )),
                ));
            }
            AdditionalCost::ExileSharingCardType { count } => {
                results.push_pay_costs(PayCost::new(
                    card,
                    Cost::ExileCardsSharingType(ExileCardsSharingType::new(None, *count)),
                ));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::{
        in_play::{CardId, Database},
        load_cards,
        pending_results::ResolutionResult,
        player::AllPlayers,
        stack::Stack,
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let mut all_players = AllPlayers::default();
        let player = all_players.new_player("Player".to_string(), 20);
        let mut db = Database::new(all_players);
        let card1 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");

        let mut results = card1.move_to_stack(&mut db, Default::default(), None, vec![]);
        let result = results.resolve(&mut db, None);
        assert_eq!(result, ResolutionResult::Complete);

        let mut results = Stack::resolve_1(&mut db);

        let result = results.resolve(&mut db, None);
        assert_eq!(result, ResolutionResult::Complete);

        assert!(db.stack.is_empty());
        assert_eq!(
            db.battlefield
                .battlefields
                .values()
                .flat_map(|b| b.iter())
                .copied()
                .collect_vec(),
            [card1]
        );

        Ok(())
    }
}
