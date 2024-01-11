use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::{
    abilities::{Ability, ForceEtbTapped, GainMana, StaticAbility},
    action_result::ActionResult,
    cost::{AdditionalCost, PayLife},
    effects::{EffectBehaviors, EffectDuration, Replacing},
    in_play::{CardId, Database, ExileReason},
    log::LogId,
    pending_results::{
        choose_targets::ChooseTargets,
        pay_costs::{
            Cost, ExileCards, ExileCardsSharingType, ExilePermanentsCmcX, PayCost,
            SacrificePermanent, SpendMana, TapPermanent, TapPermanentsPowerXOrMore,
        },
        PendingResults, Source, TargetSource,
    },
    player::{mana_pool::SpendReason, Controller, Owner},
    protogen::{
        color::Color,
        targets::Location,
        triggers::{self, TriggerSource},
    },
    stack::{ActiveTarget, Stack},
};

#[must_use]
#[derive(Debug)]
pub(crate) enum PartialAddToBattlefieldResult {
    NeedsResolution(PendingResults),
    Continue(PendingResults),
}

#[derive(Debug, Default)]
pub struct Battlefields {
    pub battlefields: IndexMap<Controller, IndexSet<CardId>>,
}

impl std::ops::Index<Owner> for Battlefields {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Owner) -> &Self::Output {
        self.battlefields.get(&Controller::from(index)).unwrap()
    }
}

impl std::ops::Index<Controller> for Battlefields {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Controller) -> &Self::Output {
        self.battlefields.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<Owner> for Battlefields {
    fn index_mut(&mut self, index: Owner) -> &mut Self::Output {
        self.battlefields
            .entry(Controller::from(index))
            .or_default()
    }
}

impl std::ops::IndexMut<Controller> for Battlefields {
    fn index_mut(&mut self, index: Controller) -> &mut Self::Output {
        self.battlefields.entry(index).or_default()
    }
}

impl Battlefields {
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.battlefields.values().all(|cards| cards.is_empty())
    }

    #[cfg(test)]
    pub(crate) fn no_modifiers(db: &Database) -> bool {
        db.modifiers.values().all(|modifier| !modifier.active)
    }

    pub fn add_from_stack_or_hand(
        db: &mut Database,
        source_card_id: CardId,
        target: Option<CardId>,
    ) -> PendingResults {
        let mut results =
            match Self::start_adding_to_battlefield(db, source_card_id, false, target, |card, _| {
                ActionResult::AddToBattlefieldSkipReplacementEffects(card, target)
            }) {
                PartialAddToBattlefieldResult::NeedsResolution(results) => return results,
                PartialAddToBattlefieldResult::Continue(results) => results,
            };

        complete_add_from_stack_or_hand(db, source_card_id, &mut results);

        results
    }

    pub(crate) fn add_from_library(
        db: &mut Database,
        source_card_id: CardId,
        enters_tapped: bool,
    ) -> PendingResults {
        let mut results = match Self::start_adding_to_battlefield(
            db,
            source_card_id,
            enters_tapped,
            None,
            |card, enters_tapped| ActionResult::AddToBattlefieldSkipReplacementEffectsFromLibrary {
                card,
                enters_tapped,
            },
        ) {
            PartialAddToBattlefieldResult::NeedsResolution(results) => return results,
            PartialAddToBattlefieldResult::Continue(results) => results,
        };

        complete_add_from_library(db, source_card_id, &mut results);

        results
    }

    pub(crate) fn add_from_exile(
        db: &mut Database,
        source_card_id: CardId,
        enters_tapped: bool,
        target: Option<CardId>,
    ) -> PendingResults {
        let mut results = match Self::start_adding_to_battlefield(
            db,
            source_card_id,
            enters_tapped,
            None,
            |card, _| ActionResult::AddToBattlefieldSkipReplacementEffectsFromExile(card, target),
        ) {
            PartialAddToBattlefieldResult::NeedsResolution(results) => return results,
            PartialAddToBattlefieldResult::Continue(results) => results,
        };

        complete_add_from_exile(db, source_card_id, &mut results);

        results
    }

    fn start_adding_to_battlefield(
        db: &mut Database,
        source_card_id: CardId,
        enters_tapped: bool,
        target: Option<CardId>,
        mut construct_skip_replacement: impl FnMut(CardId, bool) -> ActionResult,
    ) -> PartialAddToBattlefieldResult {
        let mut results = PendingResults::default();

        db[source_card_id].replacements_active = true;

        let mut replaced = false;
        for (source, replacement) in db.replacement_abilities_watching(Replacing::Etb) {
            if !source_card_id.passes_restrictions(
                db,
                LogId::current(db),
                source,
                &replacement.restrictions,
            ) {
                continue;
            }
            replaced = true;

            let controller = db[source].controller;
            for effect in replacement.effects.iter() {
                effect
                    .effect
                    .push_pending_behavior(db, source, controller, &mut results);
            }

            results.push_settled(construct_skip_replacement(source_card_id, enters_tapped));
        }

        if replaced {
            return PartialAddToBattlefieldResult::NeedsResolution(results);
        }

        move_card_to_battlefield(db, source_card_id, enters_tapped, &mut results, target);

        PartialAddToBattlefieldResult::Continue(results)
    }

    pub(crate) fn controlled_colors(db: &Database, player: Controller) -> HashSet<Color> {
        let mut colors = HashSet::default();
        for card in db.battlefield[player].as_slice() {
            colors.extend(db[*card].modified_colors.iter().copied())
        }

        colors
    }

    pub(crate) fn untap(db: &mut Database, player: Owner) {
        let cards = db
            .battlefield
            .battlefields
            .iter()
            .flat_map(|(controller, cards)| cards.iter().map(|card| (*controller, *card)))
            .filter_map(|(controller, card)| {
                if controller == player
                    || db[card].modified_static_abilities.iter().any(|ability| {
                        matches!(db[*ability].ability, StaticAbility::UntapEachUntapStep)
                    })
                {
                    Some(card)
                } else {
                    None
                }
            })
            .collect_vec();

        for card in cards {
            card.untap(db);
        }
    }

    pub(crate) fn end_turn(db: &mut Database) -> PendingResults {
        for card in db.battlefield.battlefields.values().flat_map(|b| b.iter()) {
            db.cards.entry(*card).or_default().marked_damage = 0;
        }

        let mut results = PendingResults::default();

        for card in db
            .exile
            .exile_zones
            .values()
            .flat_map(|e| e.iter())
            .copied()
            .filter(|card| db[*card].exile_duration == Some(EffectDuration::UntilEndOfTurn))
            .collect_vec()
        {
            results.extend(Battlefields::add_from_exile(db, card, false, None));
        }

        let all_modifiers = db
            .modifiers
            .iter()
            .filter_map(|(id, modifier)| {
                if modifier.active
                    && matches!(modifier.modifier.duration, EffectDuration::UntilEndOfTurn)
                {
                    Some(id)
                } else {
                    None
                }
            })
            .copied()
            .collect_vec();

        for modifier in all_modifiers {
            modifier.deactivate(db);
        }

        for card in db.cards.keys().copied().collect_vec() {
            card.apply_modifiers_layered(db);
        }

        results
    }

    pub fn check_sba(db: &mut Database) -> PendingResults {
        let mut result = PendingResults::default();
        for card in db
            .battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
        {
            let toughness = card.toughness(db);

            if toughness.is_some()
                && (toughness.unwrap() <= 0
                    || ((toughness.unwrap() - card.marked_damage(db)) <= 0
                        && !card.indestructible(db)))
            {
                result.push_settled(ActionResult::PermanentToGraveyard(card));
            }

            let enchanting = db[card].enchanting;
            if enchanting.is_some()
                && !enchanting
                    .unwrap()
                    .is_in_location(db, Location::ON_BATTLEFIELD)
            {
                result.push_settled(ActionResult::PermanentToGraveyard(card));
            }
        }

        result
    }

    pub fn activate_ability(
        db: &mut Database,
        pending: &Option<PendingResults>,
        activator: Owner,
        source: CardId,
        index: usize,
    ) -> PendingResults {
        if db.stack.split_second(db) {
            debug!("Can't activate ability (split second)");
            return PendingResults::default();
        }

        let (ability_source, ability) = db[source].abilities(db).into_iter().nth(index).unwrap();

        if !ability.can_be_activated(db, source, activator, pending) {
            debug!("Can't activate ability (can't meet costs)");
            return PendingResults::default();
        }

        let mut results = PendingResults::default();
        if let Some(cost) = ability.cost(db) {
            if cost.tap {
                if source.tapped(db) {
                    unreachable!()
                }

                results.push_settled(ActionResult::TapPermanent(source));
            }

            let exile_reason = match &ability {
                Ability::Activated(activated) => {
                    if db[*activated].ability.craft {
                        Some(ExileReason::Craft)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            for cost in cost.additional_cost.iter() {
                match cost {
                    AdditionalCost::DiscardThis => {
                        results.push_settled(ActionResult::Discard(ability_source));
                    }
                    AdditionalCost::SacrificeSource => {
                        results.push_settled(ActionResult::PermanentToGraveyard(ability_source));
                        results
                            .push_invalid_target(ActiveTarget::Battlefield { id: ability_source })
                    }
                    AdditionalCost::PayLife(PayLife { count }) => {
                        results.push_settled(ActionResult::LoseLife {
                            target: db[source].controller,
                            count: *count,
                        })
                    }
                    AdditionalCost::SacrificePermanent(restrictions) => {
                        results.push_pay_costs(PayCost::new(
                            source,
                            Cost::SacrificePermanent(SacrificePermanent::new(restrictions.clone())),
                        ));
                    }
                    AdditionalCost::TapPermanent(restrictions) => {
                        results.push_pay_costs(PayCost::new(
                            source,
                            Cost::TapPermanent(TapPermanent::new(restrictions.clone())),
                        ));
                    }
                    AdditionalCost::TapPermanentsPowerXOrMore { x_is, restrictions } => {
                        results.push_pay_costs(PayCost::new(
                            source,
                            Cost::TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore::new(
                                restrictions.clone(),
                                *x_is,
                            )),
                        ));
                    }
                    AdditionalCost::ExileCardsCmcX(restrictions) => {
                        results.push_pay_costs(PayCost::new(
                            source,
                            Cost::ExilePermanentsCmcX(ExilePermanentsCmcX::new(
                                restrictions.clone(),
                            )),
                        ));
                    }
                    AdditionalCost::ExileCard { restrictions } => {
                        results.push_pay_costs(PayCost::new(
                            source,
                            Cost::ExileCards(ExileCards::new(
                                exile_reason,
                                1,
                                1,
                                restrictions.clone(),
                            )),
                        ));
                    }
                    AdditionalCost::ExileXOrMoreCards {
                        minimum,
                        restrictions,
                    } => {
                        results.push_pay_costs(PayCost::new(
                            source,
                            Cost::ExileCards(ExileCards::new(
                                exile_reason,
                                *minimum,
                                usize::MAX,
                                restrictions.clone(),
                            )),
                        ));
                    }
                    AdditionalCost::ExileSharingCardType { count } => {
                        results.push_pay_costs(PayCost::new(
                            source,
                            Cost::ExileCardsSharingType(ExileCardsSharingType::new(
                                exile_reason,
                                *count,
                            )),
                        ));
                    }
                    AdditionalCost::RemoveCounter { counter, count } => {
                        results.push_settled(ActionResult::RemoveCounters {
                            target: source,
                            counter: *counter,
                            count: *count,
                        })
                    }
                }
            }

            results.push_pay_costs(PayCost::new(
                source,
                Cost::SpendMana(SpendMana::new(
                    cost.mana_cost.clone(),
                    SpendReason::Activating(source),
                )),
            ));
        }

        if let Ability::Mana(gain) = ability {
            if let GainMana::Choice { .. } = &db[gain].ability.gain {
                results.push_choose_mode(Source::Ability {
                    source,
                    ability: Ability::Mana(gain),
                });
            }
            results.add_gain_mana(source, gain);
        } else {
            results.add_ability_to_stack(source, ability.clone());
            let controller = db[source].controller;

            for effect in ability.effects(db) {
                let effect = effect.effect;
                let valid_targets = effect.valid_targets(
                    db,
                    source,
                    crate::log::LogId::current(db),
                    controller,
                    results.all_currently_targeted(),
                );

                if effect.needs_targets(db, source) > valid_targets.len() {
                    return PendingResults::default();
                }

                if !valid_targets.is_empty() {
                    results.push_choose_targets(ChooseTargets::new(
                        TargetSource::Effect(effect),
                        valid_targets,
                        crate::log::LogId::current(db),
                        source,
                    ));
                }
            }
        }

        results
    }

    pub(crate) fn static_abilities(db: &Database) -> Vec<(&StaticAbility, CardId)> {
        let mut result: Vec<(&StaticAbility, CardId)> = Default::default();

        for card in db
            .battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
        {
            for ability in db[card].modified_static_abilities.iter() {
                result.push((&db[*ability].ability, card));
            }
        }

        result
    }

    pub(crate) fn permanent_to_hand(db: &mut Database, target: CardId) -> PendingResults {
        target.move_to_hand(db);
        for card in db.cards.keys().copied().collect_vec() {
            card.apply_modifiers_layered(db);
        }

        PendingResults::default()
    }

    pub(crate) fn permanent_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::PUT_INTO_GRAVEYARD) {
            if matches!(
                trigger.trigger.from.enum_value().unwrap(),
                triggers::Location::ANYWHERE | triggers::Location::BATTLEFIELD
            ) && target.passes_restrictions(
                db,
                LogId::current(db),
                listener,
                &trigger.trigger.restrictions,
            ) {
                pending.extend(Stack::move_trigger_to_stack(db, listener, trigger));
            }
        }

        pending.extend(Self::leave_battlefield(db, target));
        target.move_to_graveyard(db);

        for card in db.cards.keys().copied().collect_vec() {
            card.apply_modifiers_layered(db);
        }

        pending
    }

    pub(crate) fn library_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::PUT_INTO_GRAVEYARD) {
            if matches!(
                trigger.trigger.from.enum_value().unwrap(),
                triggers::Location::ANYWHERE | triggers::Location::LIBRARY
            ) && target.passes_restrictions(
                db,
                LogId::current(db),
                listener,
                &trigger.trigger.restrictions,
            ) {
                pending.extend(Stack::move_trigger_to_stack(db, listener, trigger));
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    pub(crate) fn leave_battlefield(db: &mut Database, target: CardId) -> PendingResults {
        let mut results = PendingResults::default();

        for card in db[target]
            .exiling
            .iter()
            .copied()
            .filter(|card| {
                matches!(
                    db[*card].exile_duration,
                    Some(EffectDuration::UntilSourceLeavesBattlefield)
                )
            })
            .collect_vec()
        {
            results.extend(Battlefields::add_from_exile(db, card, false, None));
        }

        for modifier in db
            .modifiers
            .iter()
            .filter_map(|(id, modifier)| {
                if (matches!(
                    modifier.modifier.duration,
                    EffectDuration::UntilSourceLeavesBattlefield
                ) && modifier.source == target)
                    || (matches!(
                        modifier.modifier.duration,
                        EffectDuration::UntilTargetLeavesBattlefield
                    ) && modifier.modifying.contains(&target))
                {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect_vec()
        {
            modifier.deactivate(db);
        }

        db[target].left_battlefield_turn = Some(db.turn.turn_count);

        results
    }

    pub(crate) fn stack_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::PUT_INTO_GRAVEYARD) {
            if matches!(
                trigger.trigger.from.enum_value().unwrap(),
                triggers::Location::LIBRARY
            ) && target.passes_restrictions(
                db,
                LogId::current(db),
                listener,
                &trigger.trigger.restrictions,
            ) {
                pending.extend(Stack::move_trigger_to_stack(db, listener, trigger));
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    pub(crate) fn exile(
        db: &mut Database,
        source: CardId,
        target: CardId,
        reason: Option<ExileReason>,
        duration: EffectDuration,
    ) -> PendingResults {
        target.move_to_exile(db, source, reason, duration);

        let mut results = PendingResults::default();
        if let Some(ExileReason::Craft) = reason {
            for (listener, trigger) in
                db.active_triggers_of_source(TriggerSource::EXILED_DURING_CRAFT)
            {
                if matches!(
                    trigger.trigger.from.enum_value().unwrap(),
                    triggers::Location::ANYWHERE | triggers::Location::BATTLEFIELD
                ) && source.passes_restrictions(
                    db,
                    LogId::current(db),
                    listener,
                    &trigger.trigger.restrictions,
                ) {
                    results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                }
            }
        }

        results.extend(Self::leave_battlefield(db, target));
        results
    }
}

pub(crate) fn complete_add_from_library(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for (listener, trigger) in db.active_triggers_of_source(TriggerSource::ENTERS_THE_BATTLEFIELD) {
        if matches!(
            trigger.trigger.from.enum_value().unwrap(),
            triggers::Location::ANYWHERE | triggers::Location::LIBRARY
        ) && source_card_id.passes_restrictions(
            db,
            LogId::current(db),
            listener,
            &trigger.trigger.restrictions,
        ) {
            results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
        }
    }

    for card in db.cards.keys().copied().collect_vec() {
        card.apply_modifiers_layered(db);
    }
}

pub(crate) fn complete_add_from_exile(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for (listener, trigger) in db.active_triggers_of_source(TriggerSource::ENTERS_THE_BATTLEFIELD) {
        if matches!(
            trigger.trigger.from.enum_value().unwrap(),
            triggers::Location::ANYWHERE
        ) && source_card_id.passes_restrictions(
            db,
            LogId::current(db),
            listener,
            &trigger.trigger.restrictions,
        ) {
            results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
        }
    }

    for card in db.cards.keys().copied().collect_vec() {
        card.apply_modifiers_layered(db);
    }
}

pub(crate) fn complete_add_from_graveyard(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for (listener, trigger) in db.active_triggers_of_source(TriggerSource::ENTERS_THE_BATTLEFIELD) {
        if matches!(
            trigger.trigger.from.enum_value().unwrap(),
            triggers::Location::ANYWHERE
        ) && source_card_id.passes_restrictions(
            db,
            LogId::current(db),
            listener,
            &trigger.trigger.restrictions,
        ) {
            results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
        }
    }

    for card in db.cards.keys().copied().collect_vec() {
        card.apply_modifiers_layered(db);
    }
}

pub(crate) fn complete_add_from_stack_or_hand(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for (listener, trigger) in db.active_triggers_of_source(TriggerSource::ENTERS_THE_BATTLEFIELD) {
        if matches!(
            trigger.trigger.from.enum_value().unwrap(),
            triggers::Location::ANYWHERE
        ) && source_card_id.passes_restrictions(
            db,
            LogId::current(db),
            listener,
            &trigger.trigger.restrictions,
        ) {
            results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
        }
    }

    for card in db.cards.keys().copied().collect_vec() {
        card.apply_modifiers_layered(db);
    }
}

pub(crate) fn move_card_to_battlefield(
    db: &mut Database,
    source_card_id: CardId,
    enters_tapped: bool,
    results: &mut PendingResults,
    target: Option<CardId>,
) {
    if let Some(target) = target {
        target.apply_aura(db, source_card_id);
    }

    for ability in db
        .cards
        .get(&source_card_id)
        .unwrap()
        .modified_static_abilities
        .iter()
    {
        if let Some(modifier) = db[*ability].owned_modifier {
            results.push_settled(ActionResult::AddModifier { modifier })
        }
    }

    if !db[source_card_id].modified_etb_abilities.is_empty() {
        results.extend(Stack::move_etb_ability_to_stack(
            db,
            Ability::EtbOrTriggered(db[source_card_id].modified_etb_abilities.clone()),
            source_card_id,
        ));
    }

    let must_enter_tapped = Battlefields::static_abilities(db)
        .iter()
        .any(|(ability, card)| match ability {
            StaticAbility::ForceEtbTapped(ForceEtbTapped { restrictions }) => {
                source_card_id.passes_restrictions(db, LogId::current(db), *card, restrictions)
            }
            _ => false,
        });

    if must_enter_tapped || source_card_id.faceup_face(db).etb_tapped || enters_tapped {
        results.extend(source_card_id.tap(db));
    }
    source_card_id.move_to_battlefield(db);
}
