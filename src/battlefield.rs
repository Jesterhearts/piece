use std::{collections::HashSet, vec::IntoIter};

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use rand::{seq::SliceRandom, thread_rng};
use tracing::Level;

use crate::{
    abilities::{Ability, ForceEtbTapped, GainMana, StaticAbility, TriggeredAbility},
    cost::{AdditionalCost, PayLife},
    counters::Counter,
    effects::{
        battle_cry::BattleCry,
        cascade::Cascade,
        reveal_each_top_of_library::RevealEachTopOfLibrary,
        target_gains_counters::{DynamicCounter, GainCount},
        AnyEffect, BattlefieldModifier, Destination, Effect, EffectBehaviors, EffectDuration,
        ModifyBattlefield, ReplacementAbility, Replacing, Token,
    },
    in_play::{target_from_location, CardId, CastFrom, Database, ExileReason, ModifierId},
    library::Library,
    log::{Log, LogEntry, LogId},
    mana::ManaRestriction,
    pending_results::{
        choose_targets::ChooseTargets,
        examine_top_cards::{self, ExamineCards},
        pay_costs::{
            Cost, ExileCards, ExileCardsSharingType, ExilePermanentsCmcX, PayCost,
            SacrificePermanent, SpendMana, TapPermanent, TapPermanentsPowerXOrMore,
        },
        PendingResults, Source, TargetSource,
    },
    player::{
        mana_pool::{ManaSource, SpendReason},
        Controller, Owner, Player,
    },
    protogen::{color::Color, mana::Mana, types::Type},
    stack::{ActiveTarget, Entry, Stack, StackEntry, StackId},
    targets::{ControllerRestriction, Location, Restriction},
    triggers::{self, Trigger, TriggerSource},
    types::TypeSet,
};

#[must_use]
#[derive(Debug)]
pub(crate) enum PartialAddToBattlefieldResult {
    NeedsResolution(PendingResults),
    Continue(PendingResults),
}

#[derive(Debug, Clone)]
pub(crate) enum ActionResult {
    AddAbilityToStack {
        source: CardId,
        ability: Ability,
        targets: Vec<Vec<ActiveTarget>>,
        x_is: Option<usize>,
    },
    AddCounters {
        source: CardId,
        target: CardId,
        count: GainCount,
        counter: Counter,
    },
    AddModifier {
        modifier: ModifierId,
    },
    AddToBattlefield(CardId, Option<CardId>),
    AddToBattlefieldFromLibrary {
        card: CardId,
        enters_tapped: bool,
    },
    AddToBattlefieldSkipReplacementEffects(CardId, Option<CardId>),
    AddToBattlefieldSkipReplacementEffectsFromExile(CardId, Option<CardId>),
    AddToBattlefieldSkipReplacementEffectsFromLibrary {
        card: CardId,
        enters_tapped: bool,
    },
    AddTriggerToStack {
        source: CardId,
        trigger: TriggeredAbility,
        targets: Vec<Vec<ActiveTarget>>,
    },
    ApplyAuraToTarget {
        aura_source: CardId,
        target: ActiveTarget,
    },
    ApplyToBattlefield(ModifierId),
    BanAttacking(Owner),
    Cascade {
        source: CardId,
        cascading: usize,
        player: Controller,
    },
    CascadeExileToBottomOfLibrary(Controller),
    CastCard {
        card: CardId,
        targets: Vec<Vec<ActiveTarget>>,
        from: CastFrom,
        x_is: Option<usize>,
        chosen_modes: Vec<usize>,
    },
    CloneCreatureNonTargeting {
        source: CardId,
        target: ActiveTarget,
    },
    CopyAbility {
        source: CardId,
        ability: Ability,
        targets: Vec<Vec<ActiveTarget>>,
        x_is: Option<usize>,
    },
    CopyCardInStack {
        card: CardId,
        controller: Controller,
        targets: Vec<Vec<ActiveTarget>>,
        x_is: Option<usize>,
        chosen_modes: Vec<usize>,
    },
    CreateToken {
        source: CardId,
        token: Token,
    },
    CreateTokenCopyOf {
        source: CardId,
        target: CardId,
        modifiers: Vec<crate::effects::ModifyBattlefield>,
    },
    DamageTarget {
        quantity: usize,
        target: ActiveTarget,
    },
    DeclareAttackers {
        attackers: Vec<CardId>,
        targets: Vec<Owner>,
    },
    DestroyEach(CardId, Vec<Restriction>),
    DestroyTarget(ActiveTarget),
    Discard(CardId),
    DiscardCards {
        target: Controller,
        count: usize,
    },
    Discover {
        source: CardId,
        count: usize,
        player: Controller,
    },
    DrawCards {
        target: Controller,
        count: usize,
    },
    ExamineTopCards {
        destinations: IndexMap<Destination, usize>,
        count: usize,
        controller: Controller,
    },
    ExileGraveyard {
        target: ActiveTarget,
        source: CardId,
    },
    ExileTarget {
        source: CardId,
        target: ActiveTarget,
        duration: EffectDuration,
        reason: Option<ExileReason>,
    },
    Explore {
        target: ActiveTarget,
    },
    ForEachManaOfSource {
        card: CardId,
        source: ManaSource,
        effect: Effect,
    },
    GainLife {
        target: Controller,
        count: usize,
    },
    GainMana {
        gain: Vec<protobuf::EnumOrUnknown<Mana>>,
        target: Controller,
        source: ManaSource,
        restriction: ManaRestriction,
    },
    HandFromBattlefield(CardId),
    IfWasThen {
        if_was: Vec<Restriction>,
        then: Vec<Effect>,
        source: CardId,
        controller: Controller,
    },
    LoseLife {
        target: Controller,
        count: usize,
    },
    ManifestTopOfLibrary(Controller),
    Mill {
        count: usize,
        targets: Vec<ActiveTarget>,
    },
    ModifyCreatures {
        targets: Vec<ActiveTarget>,
        modifier: ModifierId,
    },
    MoveFromLibraryToBottomOfLibrary(CardId),
    MoveFromLibraryToGraveyard(CardId),
    MoveFromLibraryToTopOfLibrary(CardId),
    MoveToHandFromLibrary(CardId),
    PermanentToGraveyard(CardId),
    PlayerLoses(Owner),
    RemoveCounters {
        target: CardId,
        counter: Counter,
        count: usize,
    },
    ReturnFromBattlefieldToLibrary {
        target: ActiveTarget,
        under_cards: usize,
    },
    ReturnFromGraveyardToBattlefield {
        targets: Vec<ActiveTarget>,
    },
    ReturnFromGraveyardToHand {
        targets: Vec<ActiveTarget>,
    },
    ReturnFromGraveyardToLibrary {
        targets: Vec<ActiveTarget>,
    },
    ReturnTransformed {
        target: CardId,
        enters_tapped: bool,
    },
    RevealCard(CardId),
    RevealEachTopOfLibrary(CardId, RevealEachTopOfLibrary),
    Scry(CardId, usize),
    Shuffle(Owner),
    SpellCountered {
        index: StackId,
    },
    SpendMana {
        card: CardId,
        mana: Vec<Mana>,
        sources: Vec<ManaSource>,
        reason: SpendReason,
    },
    StackToGraveyard(CardId),
    TapPermanent(CardId),
    Transform {
        target: CardId,
    },
    Untap(CardId),
    UpdateStackEntries(Vec<StackEntry>),
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
                    .is_in_location(db, Location::Battlefield)
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

    #[instrument(skip(db), level = Level::DEBUG)]
    pub(crate) fn apply_action_results(
        db: &mut Database,
        results: &[ActionResult],
    ) -> PendingResults {
        let mut pending = PendingResults::default();

        for result in results.iter() {
            pending.extend(Self::apply_action_result(db, result));
        }

        let entries = Log::current_session(db).to_vec();
        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::OneOrMoreTapped) {
            if entries.iter().any(|entry| {
                let (_, LogEntry::Tapped { card }) = entry else {
                    return false;
                };

                card.passes_restrictions(
                    db,
                    LogId::current(db),
                    listener,
                    &trigger.trigger.restrictions,
                )
            }) {
                pending.extend(Stack::move_trigger_to_stack(db, listener, trigger));
            }
        }

        for card in db.cards.keys().copied().collect_vec() {
            card.apply_modifiers_layered(db);
        }

        pending
    }

    #[instrument(skip(db), level = Level::DEBUG)]
    fn apply_action_result(db: &mut Database, result: &ActionResult) -> PendingResults {
        match result {
            ActionResult::Discard(card) => {
                assert!(card.is_in_location(db, Location::Hand));
                card.move_to_graveyard(db);
                PendingResults::default()
            }
            ActionResult::DiscardCards { target, count } => {
                let mut pending = PendingResults::default();
                pending.push_choose_discard(db.hand[*target].iter().copied().collect_vec(), *count);
                pending
            }
            ActionResult::TapPermanent(card_id) => card_id.tap(db),
            ActionResult::PermanentToGraveyard(card_id) => {
                Self::permanent_to_graveyard(db, *card_id)
            }
            ActionResult::CopyAbility {
                source,
                ability,
                targets,
                x_is,
            } => {
                if let Some(x) = x_is {
                    db[*source].x_is = *x;
                }
                Stack::push_ability(db, *source, ability.clone(), targets.clone())
            }
            ActionResult::AddAbilityToStack {
                source,
                ability,
                targets,
                x_is,
            } => {
                if let Some(x) = x_is {
                    db[*source].x_is = *x;
                }

                let mut results = PendingResults::default();

                if let Ability::Activated(ability) = ability {
                    Log::activated(db, *source, *ability);
                    db.turn.activated_abilities.insert(*ability);

                    for (listener, trigger) in
                        db.active_triggers_of_source(TriggerSource::AbilityActivated)
                    {
                        results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                    }
                }
                results.extend(Stack::push_ability(
                    db,
                    *source,
                    ability.clone(),
                    targets.clone(),
                ));

                results
            }
            ActionResult::AddTriggerToStack {
                source,
                trigger,
                targets,
            } => Stack::push_ability(
                db,
                *source,
                Ability::EtbOrTriggered(trigger.effects.clone()),
                targets.clone(),
            ),
            ActionResult::CloneCreatureNonTargeting { source, target } => {
                if let ActiveTarget::Battlefield { id: target } = target {
                    source.clone_card(db, *target);
                }
                PendingResults::default()
            }
            ActionResult::AddModifier { modifier } => {
                modifier.activate(&mut db.modifiers);
                PendingResults::default()
            }
            ActionResult::Mill { count, targets } => {
                let mut pending = PendingResults::default();
                for target in targets {
                    let ActiveTarget::Player { id: target } = target else {
                        unreachable!()
                    };

                    for _ in 0..*count {
                        let card_id = db.all_players[*target].library.draw();
                        if let Some(card_id) = card_id {
                            pending.extend(Self::library_to_graveyard(db, card_id));
                        }
                    }
                }

                pending
            }
            ActionResult::ReturnFromGraveyardToLibrary { targets } => {
                for target in targets {
                    let ActiveTarget::Graveyard { id: target } = target else {
                        unreachable!()
                    };

                    Library::place_on_top(db, db[*target].owner, *target);
                }
                PendingResults::default()
            }
            ActionResult::ReturnFromGraveyardToBattlefield { targets } => {
                let mut pending = PendingResults::default();
                for target in targets {
                    let ActiveTarget::Graveyard { id: target } = target else {
                        unreachable!()
                    };
                    pending.extend(Self::add_from_stack_or_hand(db, *target, None));
                }

                pending
            }
            ActionResult::ReturnFromGraveyardToHand { targets } => {
                for target in targets {
                    let ActiveTarget::Battlefield { id } = target else {
                        unreachable!()
                    };

                    id.move_to_hand(db);
                }

                PendingResults::default()
            }
            ActionResult::ReturnFromBattlefieldToLibrary {
                target,
                under_cards,
            } => {
                let ActiveTarget::Battlefield { id: target } = target else {
                    unreachable!()
                };

                Library::place_under_top(db, db[*target].owner, *target, *under_cards);
                PendingResults::default()
            }
            ActionResult::LoseLife { target, count } => {
                db.all_players[*target].life_total -= *count as i32;
                PendingResults::default()
            }
            ActionResult::GainMana {
                gain,
                target,
                source,
                restriction,
            } => {
                for mana in gain {
                    db.all_players[*target].mana_pool.apply(
                        mana.enum_value().unwrap(),
                        *source,
                        *restriction,
                    )
                }
                PendingResults::default()
            }
            ActionResult::CreateToken { source, token } => {
                let mut results = PendingResults::default();

                let mut replacements = db
                    .replacement_abilities_watching(Replacing::TokenCreation)
                    .into_iter();

                let card = CardId::upload_token(db, db[*source].controller.into(), token.clone());
                create_token_copy_with_replacements(
                    db,
                    *source,
                    card,
                    &[],
                    &mut replacements,
                    &mut results,
                );

                results
            }
            ActionResult::DrawCards { target, count } => Player::draw(db, (*target).into(), *count),
            ActionResult::AddToBattlefield(card, target) => {
                Battlefields::add_from_stack_or_hand(db, *card, *target)
            }
            ActionResult::StackToGraveyard(card) => Battlefields::stack_to_graveyard(db, *card),
            ActionResult::ApplyToBattlefield(modifier) => {
                modifier.activate(&mut db.modifiers);
                PendingResults::default()
            }
            ActionResult::ExileTarget {
                source,
                target,
                duration,
                reason,
            } => {
                let Some(target) = target.id() else {
                    unreachable!()
                };
                if let EffectDuration::UntilSourceLeavesBattlefield = *duration {
                    if !source.is_in_location(db, Location::Battlefield) {
                        return PendingResults::default();
                    }
                }

                Battlefields::exile(db, *source, target, *reason, *duration)
            }
            ActionResult::DamageTarget { quantity, target } => {
                match target {
                    ActiveTarget::Battlefield { id } => {
                        id.mark_damage(db, *quantity);
                    }
                    ActiveTarget::Player { id } => {
                        db.all_players[*id].life_total -= *quantity as i32;
                    }
                    ActiveTarget::Graveyard { .. }
                    | ActiveTarget::Library { .. }
                    | ActiveTarget::Stack { .. } => unreachable!(),
                }
                PendingResults::default()
            }
            ActionResult::ManifestTopOfLibrary(player) => Player::manifest(db, (*player).into()),
            ActionResult::ModifyCreatures { targets, modifier } => {
                for target in targets {
                    let target = match target {
                        ActiveTarget::Battlefield { id } => id,
                        ActiveTarget::Graveyard { id } => id,
                        _ => unreachable!(),
                    };
                    target.apply_modifier(db, *modifier);
                }
                PendingResults::default()
            }
            ActionResult::SpellCountered { index } => {
                match &db.stack.entries.get(index).unwrap().ty {
                    Entry::Card(card) => Battlefields::stack_to_graveyard(db, *card),
                    Entry::Ability { .. } => {
                        db.stack.entries.shift_remove(index);
                        PendingResults::default()
                    }
                }
            }
            ActionResult::RemoveCounters {
                target,
                counter,
                count,
            } => {
                *db[*target].counters.entry(*counter).or_default() = db[*target]
                    .counters
                    .entry(*counter)
                    .or_default()
                    .saturating_sub(*count);
                PendingResults::default()
            }
            ActionResult::AddCounters {
                source,
                target,
                count,
                counter,
            } => {
                match count {
                    GainCount::Single => {
                        *db[*target].counters.entry(*counter).or_default() += 1;
                    }
                    GainCount::Multiple(count) => {
                        *db[*target].counters.entry(*counter).or_default() += *count;
                    }
                    GainCount::Dynamic(dynamic) => match dynamic {
                        DynamicCounter::X => {
                            let x = source.get_x(db);
                            if x > 0 {
                                *db[*target].counters.entry(*counter).or_default() += x;
                            }
                        }
                        DynamicCounter::LeftBattlefieldThisTurn { restrictions } => {
                            let cards = CardId::left_battlefield_this_turn(db);
                            let x = cards
                                .filter(|card| {
                                    card.passes_restrictions(
                                        db,
                                        LogId::current(db),
                                        *source,
                                        restrictions,
                                    )
                                })
                                .count();
                            if x > 0 {
                                *db[*target].counters.entry(*counter).or_default() += x;
                            }
                        }
                    },
                }

                PendingResults::default()
            }
            ActionResult::RevealCard(card) => {
                db[*card].revealed = true;
                PendingResults::default()
            }
            ActionResult::MoveToHandFromLibrary(card) => {
                card.move_to_hand(db);
                PendingResults::default()
            }
            ActionResult::AddToBattlefieldFromLibrary {
                card,
                enters_tapped,
            } => Battlefields::add_from_library(db, *card, *enters_tapped),
            ActionResult::Shuffle(owner) => {
                db.all_players[*owner].library.shuffle();
                PendingResults::default()
            }
            ActionResult::ApplyAuraToTarget {
                aura_source,
                target,
            } => {
                match target {
                    ActiveTarget::Battlefield { id } => {
                        id.apply_aura(db, *aura_source);
                    }
                    ActiveTarget::Graveyard { .. } => todo!(),
                    ActiveTarget::Player { .. } => todo!(),
                    _ => unreachable!(),
                };
                PendingResults::default()
            }
            ActionResult::PlayerLoses(player) => {
                db.all_players[*player].lost = true;
                PendingResults::default()
            }
            ActionResult::CopyCardInStack {
                card,
                controller,
                targets,
                x_is,
                chosen_modes,
            } => {
                let copy = card.token_copy_of(db, *controller);
                if let Some(x_is) = x_is {
                    db[copy].x_is = *x_is;
                }
                Stack::push_card(db, *card, targets.clone(), chosen_modes.clone())
            }
            ActionResult::CastCard {
                card,
                targets,
                from,
                x_is,
                chosen_modes,
            } => {
                let mut results = PendingResults::default();

                Log::cast(db, *card);

                results.extend(card.move_to_stack(
                    db,
                    targets.clone(),
                    Some(*from),
                    chosen_modes.clone(),
                ));
                if let Some(x_is) = x_is {
                    db[*card].x_is = *x_is;
                };
                card.apply_modifiers_layered(db);

                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::Cast) {
                    if card.passes_restrictions(
                        db,
                        LogId::current(db),
                        listener,
                        &trigger.trigger.restrictions,
                    ) {
                        results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                    }
                }

                let cascade = card.cascade(db);
                for _ in 0..cascade {
                    results.extend(Stack::move_trigger_to_stack(
                        db,
                        *card,
                        TriggeredAbility {
                            trigger: Trigger {
                                trigger: TriggerSource::Cast,
                                from: triggers::Location::Hand,
                                restrictions: vec![Restriction::Controller(
                                    ControllerRestriction::Self_,
                                )],
                            },
                            effects: vec![AnyEffect {
                                effect: Effect::from(Cascade),
                                oracle_text: Default::default(),
                            }],
                            oracle_text: "Cascade".to_string(),
                        },
                    ));
                }

                results
            }
            ActionResult::UpdateStackEntries(entries) => {
                db.stack.entries = entries
                    .iter()
                    .map(|e| (StackId::new(), e.clone()))
                    .collect();
                db.stack.settle();
                PendingResults::default()
            }
            ActionResult::HandFromBattlefield(card) => Self::permanent_to_hand(db, *card),
            ActionResult::RevealEachTopOfLibrary(source, reveal) => {
                let players = db.all_players.all_players();
                let revealed = players
                    .into_iter()
                    .filter_map(|player| {
                        Library::reveal_top(db, player).filter(|card| {
                            card.passes_restrictions(
                                db,
                                LogId::current(db),
                                *source,
                                &reveal.for_each.restrictions,
                            )
                        })
                    })
                    .collect_vec();

                let mut results = PendingResults::default();
                if revealed.is_empty() {
                    let controller = db[*source].controller;
                    for effect in reveal.for_each.if_none.iter() {
                        effect.push_pending_behavior(db, *source, controller, &mut results);
                    }
                } else {
                    for target in revealed {
                        for effect in reveal.for_each.effects.iter() {
                            effect.push_behavior_from_top_of_library(
                                db,
                                *source,
                                target,
                                &mut results,
                            );
                        }
                    }
                }

                results
            }
            ActionResult::ForEachManaOfSource {
                card,
                source,
                effect,
            } => {
                let mut results = PendingResults::default();
                if let Some(from_source) = db[*card].sourced_mana.get(source) {
                    for _ in 0..*from_source {
                        effect.push_pending_behavior(db, *card, db[*card].controller, &mut results);
                    }
                }

                results
            }
            ActionResult::CreateTokenCopyOf {
                source,
                target,
                modifiers,
            } => {
                let mut results = PendingResults::default();

                let mut replacements = db
                    .replacement_abilities_watching(Replacing::TokenCreation)
                    .into_iter();

                create_token_copy_with_replacements(
                    db,
                    *source,
                    *target,
                    modifiers,
                    &mut replacements,
                    &mut results,
                );

                results
            }
            ActionResult::MoveFromLibraryToTopOfLibrary(card) => {
                let owner = db[*card].owner;
                db.all_players[owner].library.remove(*card);
                Library::place_on_top(db, owner, *card);
                PendingResults::default()
            }
            ActionResult::MoveFromLibraryToBottomOfLibrary(card) => {
                let owner = db[*card].owner;
                db.all_players[owner].library.remove(*card);
                Library::place_on_bottom(db, owner, *card);
                PendingResults::default()
            }
            ActionResult::MoveFromLibraryToGraveyard(card) => Self::library_to_graveyard(db, *card),
            ActionResult::SpendMana {
                card,
                mana,
                sources,
                reason,
            } => {
                card.mana_from_source(db, sources);
                let controller = db[*card].controller;
                let spent = Player::spend_mana(db, controller.into(), mana, sources, *reason);
                assert!(
                    spent,
                    "Should have validated mana could be spent before spending."
                );
                PendingResults::default()
            }
            ActionResult::AddToBattlefieldSkipReplacementEffects(card, target) => {
                let mut results = PendingResults::default();
                move_card_to_battlefield(db, *card, false, &mut results, *target);
                complete_add_from_stack_or_hand(db, *card, &mut results);
                results
            }
            ActionResult::AddToBattlefieldSkipReplacementEffectsFromExile(card, target) => {
                let mut results = PendingResults::default();
                move_card_to_battlefield(db, *card, false, &mut results, *target);
                complete_add_from_exile(db, *card, &mut results);

                results
            }
            ActionResult::AddToBattlefieldSkipReplacementEffectsFromLibrary {
                card,
                enters_tapped,
            } => {
                let mut results = PendingResults::default();
                move_card_to_battlefield(db, *card, *enters_tapped, &mut results, None);
                complete_add_from_library(db, *card, &mut results);
                results
            }
            ActionResult::Untap(target) => {
                target.untap(db);
                PendingResults::default()
            }
            ActionResult::Cascade {
                source,
                cascading,
                player,
            } => {
                let mut results = PendingResults::default();
                results.cast_from(Some(CastFrom::Exile));

                while let Some(card) = Library::exile_top_card(
                    db,
                    (*player).into(),
                    *source,
                    Some(ExileReason::Cascade),
                ) {
                    if !card.is_land(db) && card.faceup_face(db).cost.cmc() < *cascading {
                        results.push_choose_cast(card, false, false);
                        break;
                    }
                }

                results.push_settled(ActionResult::CascadeExileToBottomOfLibrary(*player));

                results
            }
            ActionResult::Discover {
                source,
                count,
                player,
            } => {
                let mut results = PendingResults::default();
                results.cast_from(Some(CastFrom::Exile));

                while let Some(card) = Library::exile_top_card(
                    db,
                    (*player).into(),
                    *source,
                    Some(ExileReason::Cascade),
                ) {
                    if !card.is_land(db) && card.faceup_face(db).cost.cmc() < *count {
                        results.push_choose_cast(card, false, true);
                        break;
                    }
                }
                results.push_settled(ActionResult::CascadeExileToBottomOfLibrary(*player));
                results
            }
            ActionResult::CascadeExileToBottomOfLibrary(player) => {
                let mut cards = CardId::exiled_with_cascade(db);
                cards.shuffle(&mut thread_rng());

                for card in cards {
                    Library::place_on_bottom(db, (*player).into(), card);
                }
                PendingResults::default()
            }
            ActionResult::Scry(source, count) => {
                let mut cards = vec![];
                for _ in 0..*count {
                    let controller = db[*source].controller;
                    if let Some(card) = db.all_players[controller].library.draw() {
                        cards.push(card);
                    } else {
                        break;
                    }
                }

                let mut results = PendingResults::default();
                results.push_choose_scry(cards);

                results
            }
            ActionResult::ExamineTopCards {
                destinations,
                count,
                controller,
            } => {
                let mut cards = vec![];
                for _ in 0..*count {
                    if let Some(card) = db.all_players[*controller].library.draw() {
                        cards.push(card);
                    } else {
                        break;
                    }
                }

                let mut results = PendingResults::default();
                results.push_examine_top_cards(ExamineCards::new(
                    examine_top_cards::Location::Library,
                    cards,
                    destinations.clone(),
                ));

                results
            }
            ActionResult::GainLife { target, count } => {
                db.all_players[*target].life_total += *count as i32;
                *db.turn
                    .life_gained_this_turn
                    .entry(Owner::from(*target))
                    .or_default() += *count;
                PendingResults::default()
            }
            ActionResult::DeclareAttackers { attackers, targets } => {
                let mut results = PendingResults::default();
                for (attacker, target) in attackers.iter().zip(targets.iter()) {
                    db[*attacker].attacking = Some(*target);

                    let listeners = db.active_triggers_of_source(TriggerSource::Attacks);
                    debug!("attack listeners {:?}", listeners);
                    for (listener, trigger) in listeners {
                        if attacker.passes_restrictions(
                            db,
                            LogId::current(db),
                            listener,
                            &trigger.trigger.restrictions,
                        ) {
                            results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                        }
                    }

                    for _ in 0..attacker.battle_cry(db) {
                        results.extend(Stack::move_trigger_to_stack(
                            db,
                            *attacker,
                            TriggeredAbility {
                                trigger: Trigger {
                                    trigger: TriggerSource::Attacks,
                                    from: triggers::Location::Anywhere,
                                    restrictions: vec![Restriction::Controller(
                                        ControllerRestriction::Self_,
                                    )],
                                },
                                effects: vec![AnyEffect {
                                    effect: Effect::from(BattleCry),
                                    oracle_text: String::default(),
                                }],
                                oracle_text: "Battle cry".to_string(),
                            },
                        ));
                    }

                    if !attacker.vigilance(db) {
                        results.extend(attacker.tap(db));
                    }
                }
                debug!(
                    "Set number of attackers to {} in turn {}",
                    attackers.len(),
                    db.turn.turn_count
                );
                db.turn.number_of_attackers_this_turn = attackers.len();
                // TODO declare blockers
                results
            }
            ActionResult::DestroyEach(source, restrictions) => {
                let cards = db
                    .battlefield
                    .battlefields
                    .values()
                    .flat_map(|b| b.iter())
                    .copied()
                    .filter(|card| {
                        card.passes_restrictions(db, LogId::current(db), *source, restrictions)
                            && !card.indestructible(db)
                    })
                    .collect_vec();

                let mut results = PendingResults::default();
                for card in cards {
                    results.extend(Battlefields::permanent_to_graveyard(db, card));
                }

                results
            }
            ActionResult::DestroyTarget(target) => {
                let ActiveTarget::Battlefield { id } = target else {
                    unreachable!()
                };

                Battlefields::permanent_to_graveyard(db, *id)
            }
            ActionResult::Explore { target } => {
                let explorer = target.id().unwrap();
                let controller = db[explorer].controller;
                if let Some(card) = db.all_players[controller].library.draw() {
                    db[card].revealed = true;
                    if card.types_intersect(db, &TypeSet::from([Type::LAND])) {
                        card.move_to_hand(db);
                        PendingResults::default()
                    } else {
                        *db[explorer].counters.entry(Counter::P1P1).or_default() += 1;
                        let mut results = PendingResults::default();
                        results.push_choose_library_or_graveyard(card);

                        results
                    }
                } else {
                    PendingResults::default()
                }
            }
            ActionResult::ExileGraveyard { target, source } => {
                let ActiveTarget::Player { id } = target else {
                    unreachable!()
                };

                for card in db.graveyard[*id].iter().copied().collect_vec() {
                    card.move_to_exile(db, *source, None, EffectDuration::Permanently)
                }

                PendingResults::default()
            }
            ActionResult::ReturnTransformed {
                target,
                enters_tapped,
            } => {
                target.transform(db);
                let mut results = PendingResults::default();
                let location = if target.is_in_location(db, Location::Exile) {
                    Location::Exile
                } else if target.is_in_location(db, Location::Graveyard) {
                    Location::Graveyard
                } else {
                    unreachable!(
                        "unexpected location {:?}",
                        target_from_location(db, *target)
                    )
                };
                move_card_to_battlefield(db, *target, *enters_tapped, &mut results, None);
                match location {
                    Location::Exile => complete_add_from_exile(db, *target, &mut results),
                    Location::Graveyard => complete_add_from_graveyard(db, *target, &mut results),
                    _ => unreachable!(),
                }

                results
            }
            ActionResult::Transform { target } => {
                target.transform(db);

                PendingResults::default()
            }
            ActionResult::BanAttacking(player) => {
                db.turn.ban_attacking_this_turn.insert(*player);
                PendingResults::default()
            }
            ActionResult::IfWasThen {
                if_was,
                then,
                source,
                controller,
            } => {
                let mut results = PendingResults::default();
                let entries = Log::current_session(db);
                if entries
                    .iter()
                    .any(|entry| entry.1.left_battlefield_passes_restrictions(if_was))
                {
                    for effect in then.iter() {
                        effect.push_pending_behavior(db, *source, *controller, &mut results);
                    }
                }

                results
            }
        }
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

        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::PutIntoGraveyard) {
            if matches!(
                trigger.trigger.from,
                triggers::Location::Anywhere | triggers::Location::Battlefield
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

        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::PutIntoGraveyard) {
            if matches!(
                trigger.trigger.from,
                triggers::Location::Anywhere | triggers::Location::Library
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

        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::PutIntoGraveyard) {
            if matches!(trigger.trigger.from, triggers::Location::Library)
                && target.passes_restrictions(
                    db,
                    LogId::current(db),
                    listener,
                    &trigger.trigger.restrictions,
                )
            {
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
                db.active_triggers_of_source(TriggerSource::ExiledDuringCraft)
            {
                if matches!(
                    trigger.trigger.from,
                    triggers::Location::Anywhere | triggers::Location::Battlefield
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

#[instrument(skip(db, modifiers, results))]
pub(crate) fn create_token_copy_with_replacements(
    db: &mut Database,
    source: CardId,
    copying: CardId,
    modifiers: &[ModifyBattlefield],
    replacements: &mut IntoIter<(CardId, ReplacementAbility)>,
    results: &mut PendingResults,
) {
    let mut replaced = false;
    if replacements.len() > 0 {
        while let Some((source, replacement)) = replacements.next() {
            if !source.passes_restrictions(
                db,
                LogId::current(db),
                source,
                &source.faceup_face(db).restrictions,
            ) || !copying.passes_restrictions(
                db,
                LogId::current(db),
                source,
                &replacement.restrictions,
            ) {
                continue;
            }

            debug!("Replacing token creation");

            replaced = true;
            for effect in replacement.effects.iter() {
                effect.effect.replace_token_creation(
                    db,
                    source,
                    replacements,
                    copying,
                    modifiers,
                    results,
                );
            }
            break;
        }
    }

    if !replaced {
        debug!("Creating token");
        let token = copying.token_copy_of(db, db[source].controller);
        for modifier in modifiers.iter() {
            let modifier = ModifierId::upload_temporary_modifier(
                db,
                token,
                BattlefieldModifier {
                    modifier: modifier.clone(),
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                    restrictions: vec![],
                },
            );
            modifier.activate(&mut db.modifiers);

            token.apply_modifier(db, modifier);
        }

        token.apply_modifiers_layered(db);
        results.extend(Battlefields::add_from_stack_or_hand(db, token, None));
    }
}

fn complete_add_from_library(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for (listener, trigger) in db.active_triggers_of_source(TriggerSource::EntersTheBattlefield) {
        if matches!(
            trigger.trigger.from,
            triggers::Location::Anywhere | triggers::Location::Library
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

fn complete_add_from_exile(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for (listener, trigger) in db.active_triggers_of_source(TriggerSource::EntersTheBattlefield) {
        if matches!(trigger.trigger.from, triggers::Location::Anywhere)
            && source_card_id.passes_restrictions(
                db,
                LogId::current(db),
                listener,
                &trigger.trigger.restrictions,
            )
        {
            results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
        }
    }

    for card in db.cards.keys().copied().collect_vec() {
        card.apply_modifiers_layered(db);
    }
}

fn complete_add_from_graveyard(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for (listener, trigger) in db.active_triggers_of_source(TriggerSource::EntersTheBattlefield) {
        if matches!(trigger.trigger.from, triggers::Location::Anywhere)
            && source_card_id.passes_restrictions(
                db,
                LogId::current(db),
                listener,
                &trigger.trigger.restrictions,
            )
        {
            results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
        }
    }

    for card in db.cards.keys().copied().collect_vec() {
        card.apply_modifiers_layered(db);
    }
}

fn complete_add_from_stack_or_hand(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for (listener, trigger) in db.active_triggers_of_source(TriggerSource::EntersTheBattlefield) {
        if matches!(trigger.trigger.from, triggers::Location::Anywhere)
            && source_card_id.passes_restrictions(
                db,
                LogId::current(db),
                listener,
                &trigger.trigger.restrictions,
            )
        {
            results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
        }
    }

    for card in db.cards.keys().copied().collect_vec() {
        card.apply_modifiers_layered(db);
    }
}

fn move_card_to_battlefield(
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
