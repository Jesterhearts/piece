use std::collections::HashSet;

use indexmap::IndexMap;
use itertools::Itertools;
use protobuf::Enum;
use tracing::Level;
use uuid::Uuid;

use crate::{
    abilities::Ability,
    action_result::ActionResult,
    battlefield::Battlefields,
    effects::EffectBehaviors,
    in_play::{CastFrom, Database},
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
    player::mana_pool::SpendReason,
    protogen::{
        abilities::TriggeredAbility,
        cost::additional_cost::{self, ExileXOrMoreCards},
        ids::{CardId, Owner, StackId},
        keywords::Keyword,
        targets::{
            comparison,
            dynamic::Dynamic,
            restriction::{
                self, cmc::Cmc, EnteredBattlefieldThisTurn, Locations, NotKeywords, NotOfType,
                OfColor, OfType,
            },
            Location, Restriction,
        },
        triggers::TriggerSource,
    },
};

impl StackId {
    pub(crate) fn generate() -> Self {
        let (hi, lo) = Uuid::new_v4().as_u64_pair();
        Self {
            hi,
            lo,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
enum ResolutionType {
    Card(CardId),
    Ability(CardId),
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ActiveTarget {
    Stack { id: StackId },
    Battlefield { id: CardId },
    Graveyard { id: CardId },
    Library { id: CardId },
    Exile { id: CardId },
    Hand { id: CardId },
    Player { id: Owner },
}

impl ActiveTarget {
    pub(crate) fn display(&self, db: &Database) -> String {
        match self {
            ActiveTarget::Stack { id } => db.stack.entries.get(id).unwrap().display(db),
            ActiveTarget::Battlefield { id } => id.name(db).clone(),
            ActiveTarget::Graveyard { id } => id.name(db).clone(),
            ActiveTarget::Exile { id } => id.name(db).clone(),
            ActiveTarget::Library { id } => id.name(db).clone(),
            ActiveTarget::Player { id } => db.all_players[id].name.clone(),
            ActiveTarget::Hand { id } => id.name(db).clone(),
        }
    }

    pub(crate) fn id<'this, 'db>(&'this self, db: &'db Database) -> Option<&'this CardId>
    where
        'db: 'this,
    {
        match self {
            ActiveTarget::Battlefield { id }
            | ActiveTarget::Graveyard { id }
            | ActiveTarget::Library { id }
            | ActiveTarget::Hand { id }
            | ActiveTarget::Exile { id } => Some(id),
            ActiveTarget::Stack { id } => db.stack.entries.get(id).and_then(|entry| {
                if let Entry::Card(card) = &entry.ty {
                    Some(card)
                } else {
                    None
                }
            }),
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
    pub(crate) fn source(&self) -> &CardId {
        match self {
            Entry::Card(source) | Entry::Ability { source, .. } => source,
        }
    }
}

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
                format!("{}: {}", db[card_source].modified_name, ability.text(db))
            }
        }
    }

    pub(crate) fn passes_restrictions(
        &self,
        db: &Database,
        log_session: LogId,
        source: &CardId,
        restrictions: &[Restriction],
    ) -> bool {
        let spell_or_ability_controller = &db[self.ty.source()].controller;

        for restriction in restrictions.iter() {
            match restriction.restriction.as_ref().unwrap() {
                restriction::Restriction::AttackedThisTurn(_) => todo!(),
                restriction::Restriction::Attacking(_) => unreachable!(),
                restriction::Restriction::AttackingOrBlocking(_) => unreachable!(),
                restriction::Restriction::CastFromHand(_) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if !matches!(db[card].cast_from, Some(CastFrom::Hand)) {
                        return false;
                    }
                }
                restriction::Restriction::Chosen(_) => {
                    return false;
                }
                restriction::Restriction::Cmc(cmc_test) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };
                    let cmc = db[card].modified_cost.cmc() as i32;

                    match cmc_test.cmc.as_ref().unwrap() {
                        Cmc::Comparison(comparison) => {
                            let matches = match comparison.value.as_ref().unwrap() {
                                comparison::Value::LessThan(i) => cmc < i.value,
                                comparison::Value::LessThanOrEqual(i) => cmc <= i.value,
                                comparison::Value::GreaterThan(i) => cmc > i.value,
                                comparison::Value::GreaterThanOrEqual(i) => cmc >= i.value,
                            };
                            if !matches {
                                return false;
                            }
                        }
                        Cmc::Dynamic(dy) => match dy.dynamic.as_ref().unwrap() {
                            Dynamic::X(_) => {
                                if source.get_x(db) as i32 != cmc {
                                    return false;
                                }
                            }
                        },
                    }
                }
                restriction::Restriction::Controller(controller_restriction) => {
                    match controller_restriction.controller.as_ref().unwrap() {
                        restriction::controller::Controller::Self_(_) => {
                            if db[source].controller != *spell_or_ability_controller {
                                return false;
                            }
                        }
                        restriction::controller::Controller::Opponent(_) => {
                            if db[source].controller == *spell_or_ability_controller {
                                return false;
                            }
                        }
                    };
                }
                restriction::Restriction::ControllerControlsColors(colors) => {
                    let controlled_colors =
                        Battlefields::controlled_colors(db, spell_or_ability_controller);
                    if !colors
                        .colors
                        .iter()
                        .any(|color| controlled_colors.contains(&color.enum_value().unwrap()))
                    {
                        return false;
                    }
                }
                restriction::Restriction::ControllerHandEmpty(_) => {
                    if spell_or_ability_controller.has_cards(db, Location::IN_HAND) {
                        return false;
                    }
                }
                restriction::Restriction::ControllerJustCast(_) => {
                    if !Log::session(db, log_session).iter().any(|(_, entry)| {
                        let LogEntry::Cast { card } = entry else {
                            return false;
                        };
                        db[card].controller == *spell_or_ability_controller
                    }) {
                        return false;
                    }
                }
                restriction::Restriction::Descend(count) => {
                    let cards = db.graveyard[spell_or_ability_controller]
                        .iter()
                        .filter(|card| card.is_permanent(db))
                        .count() as i32;
                    if cards < count.count {
                        return false;
                    }
                }
                restriction::Restriction::DescendedThisTurn(_) => {
                    let descended = db
                        .graveyard
                        .descended_this_turn
                        .get(&Owner::from(spell_or_ability_controller.clone()))
                        .copied()
                        .unwrap_or_default();
                    if descended < 1 {
                        return false;
                    }
                }
                restriction::Restriction::DuringControllersTurn(_) => {
                    if *spell_or_ability_controller != db.turn.active_player() {
                        return false;
                    }
                }
                restriction::Restriction::EnteredBattlefieldThisTurn(
                    EnteredBattlefieldThisTurn {
                        count,
                        restrictions,
                        ..
                    },
                ) => {
                    let entered_this_turn = CardId::entered_battlefield_this_turn(db)
                        .filter(|card| {
                            card.passes_restrictions(db, log_session, source, restrictions)
                        })
                        .count() as i32;
                    if entered_this_turn < *count {
                        return false;
                    }
                }
                restriction::Restriction::HasActivatedAbility(_) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if db[card].modified_activated_abilities.is_empty() {
                        return false;
                    }
                }
                restriction::Restriction::IsPermanent(_) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if !card.is_permanent(db) {
                        return false;
                    }
                }
                restriction::Restriction::InGraveyard(_) => {
                    return false;
                }
                restriction::Restriction::JustDiscarded(_) => {
                    return false;
                }
                restriction::Restriction::LifeGainedThisTurn(count) => {
                    let gained_this_turn =
                        db.all_players[spell_or_ability_controller].life_gained_this_turn;
                    if gained_this_turn < count.count {
                        return false;
                    }
                }
                restriction::Restriction::Location(Locations { locations, .. }) => {
                    if locations.iter().any(|location| {
                        !matches!(location.enum_value().unwrap(), Location::IN_STACK)
                    }) {
                        return false;
                    }
                }
                restriction::Restriction::ManaSpentFromSource(source) => {
                    let Entry::Card(card) = &self.ty else {
                        // TODO: Pretty sure there are some mana sources that copy abilities if used.
                        return false;
                    };

                    if !db[card]
                        .sourced_mana
                        .contains_key(&source.source.enum_value().unwrap())
                    {
                        return false;
                    }
                }
                restriction::Restriction::NonToken(_) => {
                    if !matches!(self.ty, Entry::Card(_)) {
                        return false;
                    }
                }
                restriction::Restriction::NotChosen(_) => {
                    let Entry::Card(candidate) = &self.ty else {
                        return false;
                    };

                    if Log::session(db, log_session).iter().any(|(_, entry)| {
                        let LogEntry::CardChosen { card } = entry else {
                            return false;
                        };
                        card == candidate
                    }) {
                        return false;
                    }
                }
                restriction::Restriction::NotKeywords(NotKeywords { keywords, .. }) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if db[card]
                        .modified_keywords
                        .keys()
                        .any(|keyword| keywords.contains_key(keyword))
                    {
                        return false;
                    }
                }
                restriction::Restriction::NotOfType(NotOfType {
                    types, subtypes, ..
                }) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if !types.is_empty()
                        && db[card]
                            .modified_types
                            .iter()
                            .any(|ty| types.contains_key(&ty.value()))
                    {
                        return false;
                    }
                    if !subtypes.is_empty()
                        && db[card]
                            .modified_subtypes
                            .iter()
                            .any(|ty| subtypes.contains_key(&ty.value()))
                    {
                        return false;
                    }
                }
                restriction::Restriction::NotSelf(_) => {
                    let Entry::Card(card) = &self.ty else {
                        continue;
                    };

                    if source == card {
                        return false;
                    }
                }
                restriction::Restriction::NumberOfCountersOnThis { .. } => {
                    return false;
                }
                restriction::Restriction::OfColor(OfColor { colors, .. }) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if !colors.iter().any(|color| {
                        db[card]
                            .modified_colors
                            .contains(&color.enum_value().unwrap())
                    }) {
                        return false;
                    }
                }
                restriction::Restriction::OfType(OfType {
                    types, subtypes, ..
                }) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if !types.is_empty()
                        && !db[card]
                            .modified_types
                            .iter()
                            .any(|ty| types.contains_key(&ty.value()))
                    {
                        return false;
                    }
                    if !subtypes.is_empty()
                        && !db[card]
                            .modified_subtypes
                            .iter()
                            .any(|ty| subtypes.contains_key(&ty.value()))
                    {
                        return false;
                    }
                }
                restriction::Restriction::OnBattlefield(_) => {
                    return false;
                }
                restriction::Restriction::Power(comparison) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if let Some(power) = card.power(db) {
                        if !match comparison.comparison.value.as_ref().unwrap() {
                            comparison::Value::LessThan(target) => power < target.value,
                            comparison::Value::LessThanOrEqual(target) => power <= target.value,
                            comparison::Value::GreaterThan(target) => power > target.value,
                            comparison::Value::GreaterThanOrEqual(target) => power >= target.value,
                        } {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                restriction::Restriction::Self_(_) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if card != source {
                        return false;
                    }
                }
                restriction::Restriction::SourceCast(_) => {
                    if db[source].cast_from.is_none() {
                        return false;
                    }
                }
                restriction::Restriction::SpellOrAbilityJustCast(_) => {
                    match &self.ty {
                        Entry::Card(candidate) => {
                            if !Log::session(db, log_session).iter().any(|(_, entry)| {
                                if let LogEntry::Cast { card } = entry {
                                    card == candidate
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
                restriction::Restriction::Tapped(_) => {
                    return false;
                }
                restriction::Restriction::TargetedBy(_) => {
                    if !self
                        .targets
                        .iter()
                        .flat_map(|t| t.iter())
                        .flat_map(|t| t.id(db))
                        .all(|card| card == source)
                    {
                        return false;
                    }
                }
                restriction::Restriction::Threshold(_) => {
                    if db.graveyard[spell_or_ability_controller].len() < 7 {
                        return false;
                    }
                }
                restriction::Restriction::Toughness(comparison) => {
                    let Entry::Card(card) = &self.ty else {
                        return false;
                    };

                    if let Some(toughness) = card.toughness(db) {
                        if !match comparison.comparison.value.as_ref().unwrap() {
                            comparison::Value::LessThan(target) => toughness < target.value,
                            comparison::Value::LessThanOrEqual(target) => toughness <= target.value,
                            comparison::Value::GreaterThan(target) => toughness > target.value,
                            comparison::Value::GreaterThanOrEqual(target) => {
                                toughness >= target.value
                            }
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
    pub(crate) fn contains(&self, card: &CardId) -> bool {
        self.entries
            .iter()
            .any(|(_, entry)| matches!(&entry.ty, Entry::Card(entry) if entry == card))
    }

    pub(crate) fn find(&self, card: &CardId) -> Option<StackId> {
        self.entries
            .iter()
            .rev()
            .find(|(_, entry)| match &entry.ty {
                Entry::Card(entry) => entry == card,
                Entry::Ability { source, .. } => source == card,
            })
            .map(|(id, _)| id.clone())
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
            db[card]
                .modified_keywords
                .contains_key(&Keyword::SPLIT_SECOND.value())
        } else {
            false
        }
    }

    pub(crate) fn remove(&mut self, card: &CardId) {
        self.entries
            .retain(|_, entry| !matches!(&entry.ty, Entry::Card(entry) if entry == card));
    }

    #[cfg(test)]
    pub(crate) fn target_nth(&self, nth: usize) -> ActiveTarget {
        let id = self.entries.get_index(nth).unwrap().0;
        ActiveTarget::Stack { id: id.clone() }
    }

    pub fn entries(&self) -> &IndexMap<StackId, StackEntry> {
        &self.entries
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

        let (effects, controller, resolving_card, source, ty) = match next.ty {
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
                    effects,
                    db[&card].controller.clone(),
                    Some(card.clone()),
                    card.clone(),
                    ResolutionType::Card(card),
                )
            }
            Entry::Ability { source, ability } => (
                ability.effects(db),
                db[&source].controller.clone(),
                None,
                source.clone(),
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
            effect.effect.unwrap().push_behavior_with_targets(
                db,
                targets,
                &source,
                &controller,
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
            ResolutionType::Ability(source) => Log::ability_resolved(db, &source),
        }

        results
    }

    pub(crate) fn move_ability_to_stack(
        db: &mut Database,
        ability: Ability,
        source: CardId,
    ) -> PendingResults {
        let mut results = PendingResults::default();

        if ability
            .effects(db)
            .iter()
            .any(|effect| effect.effect.as_ref().unwrap().wants_targets(db, &source) > 0)
        {
            let controller = db[&source].controller.clone();
            for effect in ability.effects(db).into_iter() {
                effect.effect.unwrap().push_pending_behavior(
                    db,
                    &source,
                    &controller,
                    &mut results,
                );
            }
            results.add_ability_to_stack(source, ability);
        } else {
            results.push_settled(ActionResult::AddAbilityToStack {
                ability,
                source,
                targets: vec![],
                x_is: None,
            });
        }

        results
    }

    pub(crate) fn move_trigger_to_stack(
        db: &mut Database,
        listener: CardId,
        trigger: TriggeredAbility,
    ) -> PendingResults {
        Self::move_ability_to_stack(db, Ability::EtbOrTriggered(trigger.effects), listener)
    }

    pub(crate) fn move_card_to_stack_from_hand(
        db: &mut Database,
        card: CardId,
        paying_costs: bool,
    ) -> PendingResults {
        add_card_to_stack(db, card, Some(CastFrom::Hand), paying_costs)
    }

    pub(crate) fn push_card(
        db: &mut Database,
        source: &CardId,
        targets: Vec<Vec<ActiveTarget>>,
        chosen_modes: Vec<usize>,
    ) -> PendingResults {
        db.stack.entries.insert(
            StackId::generate(),
            StackEntry {
                ty: Entry::Card(source.clone()),
                targets: targets.clone(),
                settled: true,
                mode: chosen_modes,
            },
        );

        let mut results = PendingResults::default();
        for target in targets.into_iter().flat_map(|t| t.into_iter()) {
            if let ActiveTarget::Battlefield { id } = target {
                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::TARGETED) {
                    if listener == id
                        && source.passes_restrictions(
                            db,
                            LogId::current(db),
                            &listener,
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
        source: &CardId,
        ability: Ability,
        targets: Vec<Vec<ActiveTarget>>,
    ) -> PendingResults {
        db.stack.entries.insert(
            StackId::generate(),
            StackEntry {
                ty: Entry::Ability {
                    source: source.clone(),
                    ability,
                },
                targets: targets.clone(),
                mode: vec![],
                settled: true,
            },
        );

        let mut results = PendingResults::default();
        for target in targets.into_iter().flat_map(|t| t.into_iter()) {
            if let ActiveTarget::Battlefield { id } = target {
                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::TARGETED) {
                    if listener == id
                        && source.passes_restrictions(
                            db,
                            LogId::current(db),
                            &listener,
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

    db[&card].cast_from = from;
    card.apply_modifiers_layered(db);

    if card.has_modes(db) {
        results.push_choose_mode(Source::Card(card.clone()));
    }

    results.add_card_to_stack(card.clone(), from);
    if card.wants_targets(db).into_iter().sum::<usize>() > 0 {
        let controller = &db[&card].controller;
        if card.faceup_face(db).enchant.is_some() {
            results.push_choose_targets(ChooseTargets::new(
                TargetSource::Aura(card.clone()),
                card.targets_for_aura(db).unwrap(),
                crate::log::LogId::current(db),
                card.clone(),
            ))
        }

        if card.faceup_face(db).effects.len() == 1 {
            let effect = card
                .faceup_face(db)
                .effects
                .iter()
                .exactly_one()
                .unwrap()
                .effect
                .as_ref()
                .unwrap();
            let valid_targets = effect.valid_targets(
                db,
                &card,
                crate::log::LogId::current(db),
                controller,
                &HashSet::default(),
            );
            if valid_targets.len() < effect.needs_targets(db, &card) {
                debug!("Insufficient targets");
                return PendingResults::default();
            }

            results.push_choose_targets(ChooseTargets::new(
                TargetSource::Effect(effect.clone()),
                valid_targets,
                crate::log::LogId::current(db),
                card.clone(),
            ));
        } else {
            for effect in card.faceup_face(db).effects.iter() {
                let effect = effect.effect.as_ref().unwrap();
                let valid_targets = effect.valid_targets(
                    db,
                    &card,
                    crate::log::LogId::current(db),
                    controller,
                    &HashSet::default(),
                );
                if valid_targets.len() < effect.needs_targets(db, &card) {
                    debug!("Insufficient targets");
                    return PendingResults::default();
                }

                results.push_choose_targets(ChooseTargets::new(
                    TargetSource::Effect(effect.clone()),
                    valid_targets,
                    crate::log::LogId::current(db),
                    card.clone(),
                ));
            }
        }
    }

    // It is important that paying costs happens last, because some cards have effects that depend on what they are targeting.
    let cost = &card.faceup_face(db).cost;
    if paying_costs {
        results.push_pay_costs(PayCost::new(
            card.clone(),
            Cost::SpendMana(SpendMana::new(
                cost.mana_cost.clone(),
                SpendReason::Casting(card.clone()),
            )),
        ));
    }
    for cost in cost.additional_costs.iter() {
        match cost.cost.as_ref().unwrap() {
            additional_cost::Cost::DiscardThis(_) => unreachable!(),
            additional_cost::Cost::SacrificeSource(_) => unreachable!(),
            additional_cost::Cost::RemoveCounters(_) => unreachable!(),
            additional_cost::Cost::PayLife(_) => todo!(),
            additional_cost::Cost::SacrificePermanent(sac) => {
                results.push_pay_costs(PayCost::new(
                    card.clone(),
                    Cost::SacrificePermanent(SacrificePermanent::new(sac.restrictions.clone())),
                ));
            }
            additional_cost::Cost::TapPermanent(tap) => {
                results.push_pay_costs(PayCost::new(
                    card.clone(),
                    Cost::TapPermanent(TapPermanent::new(tap.restrictions.clone())),
                ));
            }
            additional_cost::Cost::TapPermanentsPowerXOrMore(tap) => {
                results.push_pay_costs(PayCost::new(
                    card.clone(),
                    Cost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore::new(
                        tap.restrictions.clone(),
                        tap.x_is as usize,
                    )),
                ));
            }
            additional_cost::Cost::ExileCardsCmcX(exile) => {
                results.push_pay_costs(PayCost::new(
                    card.clone(),
                    Cost::ExilePermanentsCmcX(ExilePermanentsCmcX::new(exile.restrictions.clone())),
                ));
            }
            additional_cost::Cost::ExileCard(exile) => {
                results.push_pay_costs(PayCost::new(
                    card.clone(),
                    Cost::ExileCards(ExileCards::new(None, 1, 1, exile.restrictions.clone())),
                ));
            }
            additional_cost::Cost::ExileXOrMoreCards(ExileXOrMoreCards {
                minimum,
                restrictions,
                ..
            }) => {
                results.push_pay_costs(PayCost::new(
                    card.clone(),
                    Cost::ExileCards(ExileCards::new(
                        None,
                        *minimum as usize,
                        usize::MAX,
                        restrictions.clone(),
                    )),
                ));
            }
            additional_cost::Cost::ExileSharingCardType(exile) => {
                results.push_pay_costs(PayCost::new(
                    card.clone(),
                    Cost::ExileCardsSharingType(ExileCardsSharingType::new(
                        None,
                        exile.count as usize,
                    )),
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
        in_play::Database, load_cards, pending_results::ResolutionResult, player::AllPlayers,
        protogen::ids::CardId, stack::Stack,
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let mut all_players = AllPlayers::default();
        let player = all_players.new_player("Player".to_string(), 20);
        let mut db = Database::new(all_players);
        let card1 = CardId::upload(&mut db, &cards, player.clone(), "Alpine Grizzly");

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
                .cloned()
                .collect_vec(),
            [card1]
        );

        Ok(())
    }
}
