pub mod pending_results;

use std::{collections::HashSet, vec::IntoIter};

use bevy_ecs::{entity::Entity, query::With};

use indexmap::IndexSet;
use itertools::Itertools;
use rand::{seq::SliceRandom, thread_rng};
use tracing::Level;

use crate::{
    abilities::{Ability, ForceEtbTapped, GainMana, StaticAbility, TriggeredAbility},
    battlefield::{
        choose_targets::ChooseTargets,
        pay_costs::{
            ExileCards, ExileCardsSharingType, ExilePermanentsCmcX, PayCost, SacrificePermanent,
            SpendMana, TapPermanent, TapPermanentsPowerXOrMore,
        },
    },
    card::Color,
    controller::ControllerRestriction,
    cost::{AdditionalCost, PayLife},
    effects::{
        battle_cry::BattleCry,
        cascade::Cascade,
        effect_duration::UntilEndOfTurn,
        replacing,
        reveal_each_top_of_library::RevealEachTopOfLibrary,
        target_gains_counters::{Counter, DynamicCounter, GainCount},
        AnyEffect, BattlefieldModifier, Effect, EffectDuration, ModifyBattlefield, Token,
    },
    in_play::{
        self, all_cards, cards, update_life_gained_this_turn, AbilityId, Active, AuraId, CardId,
        CastFrom, CounterId, Database, ExileReason, InExile, InGraveyard, InHand, InLibrary,
        InStack, ModifierId, NumberOfAttackers, OnBattlefield, ReplacementEffectId, TriggerId,
    },
    mana::{Mana, ManaRestriction},
    player::{
        mana_pool::{ManaSource, SpendReason},
        AllPlayers, Controller, Owner,
    },
    stack::{ActiveTarget, Entry, Stack, StackEntry},
    targets::Restriction,
    triggers::{self, trigger_source, Trigger, TriggerSource},
    turns::Turn,
    types::Type,
};

pub use pending_results::*;

#[must_use]
#[derive(Debug)]
pub enum PartialAddToBattlefieldResult {
    NeedsResolution(PendingResults),
    Continue(PendingResults),
}

#[derive(Debug, Clone)]
pub enum ActionResult {
    AddAbilityToStack {
        source: CardId,
        ability: AbilityId,
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
        trigger: TriggerId,
        source: CardId,
        targets: Vec<Vec<ActiveTarget>>,
    },
    ApplyAuraToTarget {
        aura: AuraId,
        target: ActiveTarget,
    },
    ApplyToBattlefield(ModifierId),
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
    Discover {
        source: CardId,
        count: usize,
        player: Controller,
    },
    DrawCards {
        target: Controller,
        count: usize,
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
        target: crate::player::Controller,
        count: usize,
    },
    GainMana {
        gain: Vec<Mana>,
        target: Controller,
        source: ManaSource,
        restriction: ManaRestriction,
    },
    HandFromBattlefield(CardId),
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
    MoveFromLibraryToTopOfLibrary(CardId),
    MoveToHandFromLibrary(CardId),
    PermanentToGraveyard(CardId),
    PlayerLoses(Owner),
    ReturnFromBattlefieldToLibrary {
        target: ActiveTarget,
    },
    ReturnFromGraveyardToBattlefield {
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
        id: Entry,
    },
    SpendMana {
        card: CardId,
        mana: Vec<Mana>,
        sources: Vec<ManaSource>,
        reason: SpendReason,
    },
    StackToGraveyard(CardId),
    TapPermanent(CardId),
    Untap(CardId),
    UpdateStackEntries(Vec<StackEntry>),
    Transform {
        target: crate::in_play::CardId,
    },
    ReturnFromGraveyardToHand {
        targets: Vec<ActiveTarget>,
    },
    Discard(CardId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierSource {
    UntilEndOfTurn,
    Card(CardId),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Permanent {
    pub tapped: bool,
}

#[derive(Debug)]
pub struct Battlefield;

impl Battlefield {
    pub fn is_empty(db: &mut Database) -> bool {
        db.query_filtered::<(), With<OnBattlefield>>()
            .iter(db)
            .next()
            .is_none()
    }

    pub fn no_modifiers(db: &mut Database) -> bool {
        db.modifiers
            .query_filtered::<Entity, With<Active>>()
            .iter(&db.modifiers)
            .next()
            .is_none()
    }

    pub fn number_of_cards_in_graveyard(db: &mut Database, player: Controller) -> usize {
        let mut query = db.query_filtered::<&Controller, With<InGraveyard>>();

        let mut count = 0;
        for controller in query.iter(db) {
            if player == *controller {
                count += 1
            }
        }

        count
    }

    pub fn creatures(db: &mut Database) -> Vec<CardId> {
        let on_battlefield = cards::<OnBattlefield>(db);

        let mut results = vec![];

        for card in on_battlefield.into_iter() {
            let types = card.types(db);
            if types.contains(&Type::Creature) {
                results.push(card);
            }
        }

        results
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

    pub fn add_from_library(
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

    pub fn add_from_exile(
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
        construct_skip_replacement: impl FnOnce(CardId, bool) -> ActionResult,
    ) -> PartialAddToBattlefieldResult {
        let mut results = PendingResults::default();

        ReplacementEffectId::activate_all_for_card(db, source_card_id);
        for replacement in ReplacementEffectId::watching::<replacing::Etb>(db) {
            let source = replacement.source(db);
            let restrictions = replacement.restrictions(db);
            let controller_restriction = replacement.controller_restriction(db);
            if !source_card_id.passes_restrictions(
                db,
                source,
                controller_restriction,
                &restrictions,
            ) {
                continue;
            }

            let controller = replacement.source(db).controller(db);
            for effect in replacement.effects(db) {
                let effect = effect.into_effect(db, controller);
                effect.push_pending_behavior(db, source, controller, &mut results);
            }

            source_card_id.apply_modifiers_layered(db);
            results.push_settled(construct_skip_replacement(source_card_id, enters_tapped));
            return PartialAddToBattlefieldResult::NeedsResolution(results);
        }

        move_card_to_battlefield(db, source_card_id, enters_tapped, &mut results, target);

        PartialAddToBattlefieldResult::Continue(results)
    }

    pub fn compute_deck_targets(
        db: &mut Database,
        player: Controller,
        restrictions: &[Restriction],
    ) -> Vec<CardId> {
        let mut results = vec![];

        for card in player.get_cards_in::<InLibrary>(db) {
            if !card.passes_restrictions(db, card, ControllerRestriction::You, restrictions) {
                continue;
            }

            results.push(card);
        }

        results
    }

    pub fn compute_graveyard_targets(
        db: &mut Database,
        controller: ControllerRestriction,
        source_card: CardId,
        types: &IndexSet<Type>,
    ) -> Vec<CardId> {
        let targets = match controller {
            ControllerRestriction::Any => AllPlayers::all_players_in_db(db),
            ControllerRestriction::You => HashSet::from([source_card.controller(db).into()]),
            ControllerRestriction::Opponent => {
                let mut all = AllPlayers::all_players_in_db(db);
                all.remove(&source_card.controller(db).into());
                all
            }
        };
        let mut target_cards = vec![];

        for target in targets.into_iter() {
            let cards_in_graveyard = target.get_cards::<InGraveyard>(db);
            for card in cards_in_graveyard {
                if card.types_intersect(db, types) {
                    target_cards.push(card);
                }
            }
        }

        target_cards
    }

    pub fn controlled_colors(db: &mut Database, player: Controller) -> HashSet<Color> {
        let cards = player.get_cards_in::<OnBattlefield>(db);

        let mut colors = HashSet::default();
        for card in cards {
            let card_colors = card.colors(db);
            colors.extend(card_colors)
        }

        colors
    }

    pub fn untap(db: &mut Database, player: Owner) {
        let cards = in_play::cards::<OnBattlefield>(db)
            .into_iter()
            .filter(|card| {
                card.controller(db) == player
                    || card
                        .static_abilities(db)
                        .into_iter()
                        .any(|ability| matches!(ability, StaticAbility::UntapEachUntapStep))
            })
            .collect_vec();
        for card in cards {
            card.untap(db);
        }
    }

    pub fn end_turn(db: &mut Database) -> PendingResults {
        let cards = cards::<OnBattlefield>(db);
        for card in cards {
            card.clear_damage(db);
        }

        let mut results = PendingResults::default();

        for card in in_play::cards::<InExile>(db)
            .into_iter()
            .filter(|card| card.until_end_of_turn(db))
            .collect_vec()
        {
            results.extend(Battlefield::add_from_exile(db, card, false, None));
        }

        let all_modifiers = db
            .modifiers
            .query_filtered::<Entity, (With<Active>, With<UntilEndOfTurn>)>()
            .iter(&db.modifiers)
            .map(ModifierId::from)
            .collect_vec();

        for modifier in all_modifiers {
            modifier.detach_all(db);
        }

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        results
    }

    pub fn check_sba(db: &mut Database) -> PendingResults {
        let mut result = PendingResults::default();
        for card_id in cards::<OnBattlefield>(db) {
            let toughness = card_id.toughness(db);
            debug!("Checking damage for {}", card_id.name(db));
            debug!(
                "toughness {:?}, damage {:?}",
                toughness,
                card_id.marked_damage(db)
            );

            if toughness.is_some() && (toughness.unwrap() - card_id.marked_damage(db)) <= 0 {
                result.push_settled(ActionResult::PermanentToGraveyard(card_id));
            }

            let aura = card_id.aura(db);
            if aura.is_some() && !aura.unwrap().is_attached(db) {
                result.push_settled(ActionResult::PermanentToGraveyard(card_id));
            }
        }

        result
    }

    pub fn activate_ability(
        db: &mut Database,
        all_players: &mut AllPlayers,
        turn: &Turn,
        activator: Owner,
        card: CardId,
        index: usize,
    ) -> PendingResults {
        if Stack::split_second(db) {
            debug!("Can't activate ability (split second)");
            return PendingResults::default();
        }

        let ability_id = card.activated_abilities(db)[index];

        if !ability_id.can_be_activated(db, all_players, turn, activator) {
            debug!("Can't activate ability (can't meet costs)");
            return PendingResults::default();
        }

        let ability = ability_id.ability(db);
        let mut results = PendingResults::default();
        if let Some(cost) = ability.cost() {
            if cost.tap {
                if card.tapped(db) {
                    unreachable!()
                }

                results.push_settled(ActionResult::TapPermanent(card));
            }

            let exile_reason = match &ability {
                Ability::Activated(activated) => {
                    if activated.craft {
                        Some(ExileReason::Craft)
                    } else {
                        None
                    }
                }
                Ability::Mana(_) | Ability::ETB { .. } => None,
            };

            for cost in cost.additional_cost.iter() {
                match cost {
                    AdditionalCost::DiscardThis => {
                        results.push_settled(ActionResult::Discard(card));
                    }
                    AdditionalCost::SacrificeSource => {
                        results.push_settled(ActionResult::PermanentToGraveyard(
                            ability_id.source(db),
                        ));
                        results.push_invalid_target(ActiveTarget::Battlefield {
                            id: ability_id.source(db),
                        })
                    }
                    AdditionalCost::PayLife(PayLife { count }) => {
                        results.push_settled(ActionResult::LoseLife {
                            target: card.controller(db),
                            count: *count,
                        })
                    }
                    AdditionalCost::SacrificePermanent(restrictions) => {
                        results.push_pay_costs(PayCost::SacrificePermanent(
                            SacrificePermanent::new(restrictions.clone(), card),
                        ));
                    }
                    AdditionalCost::TapPermanent(restrictions) => {
                        results.push_pay_costs(PayCost::TapPermanent(TapPermanent::new(
                            restrictions.clone(),
                            card,
                        )));
                    }
                    AdditionalCost::TapPermanentsPowerXOrMore { x_is, restrictions } => {
                        results.push_pay_costs(PayCost::TapPermanentsPowerXOrMore(
                            TapPermanentsPowerXOrMore::new(restrictions.clone(), *x_is, card),
                        ));
                    }
                    AdditionalCost::ExileCardsCmcX(restrictions) => {
                        results.push_pay_costs(PayCost::ExilePermanentsCmcX(
                            ExilePermanentsCmcX::new(restrictions.clone(), card),
                        ));
                    }
                    AdditionalCost::ExileCard { restrictions } => {
                        results.push_pay_costs(PayCost::ExileCards(ExileCards::new(
                            exile_reason,
                            1,
                            1,
                            restrictions.clone(),
                            card,
                        )));
                    }
                    AdditionalCost::ExileXOrMoreCards {
                        minimum,
                        restrictions,
                    } => {
                        results.push_pay_costs(PayCost::ExileCards(ExileCards::new(
                            exile_reason,
                            *minimum,
                            usize::MAX,
                            restrictions.clone(),
                            card,
                        )));
                    }
                    AdditionalCost::ExileSharingCardType { count } => {
                        results.push_pay_costs(PayCost::ExileCardsSharingType(
                            ExileCardsSharingType::new(exile_reason, card, *count),
                        ));
                    }
                }
            }

            results.push_pay_costs(PayCost::SpendMana(SpendMana::new(
                cost.mana_cost.clone(),
                card,
                SpendReason::Activating(ability_id),
            )));
        }

        if let Ability::Mana(gain) = ability {
            results.add_gain_mana(ability_id);
            if let GainMana::Choice { .. } = gain.gain {
                results.push_choose_mode(Source::Ability(ability_id));
            }
        } else {
            results.add_ability_to_stack(ability_id);
            let controller = card.controller(db);

            for effect in ability.into_effects() {
                let effect = effect.into_effect(db, controller);
                let valid_targets =
                    effect.valid_targets(db, card, controller, results.all_currently_targeted());

                if effect.needs_targets() > valid_targets.len() {
                    return PendingResults::default();
                }

                if !valid_targets.is_empty() {
                    results.push_choose_targets(ChooseTargets::new(
                        TargetSource::Effect(effect),
                        valid_targets,
                        card,
                    ));
                }
            }
        }

        results
    }

    pub fn static_abilities(db: &mut Database) -> Vec<(StaticAbility, Controller)> {
        let mut result: Vec<(StaticAbility, Controller)> = Default::default();

        for card in cards::<OnBattlefield>(db) {
            let controller = card.controller(db);
            for ability in card.static_abilities(db) {
                result.push((ability, controller));
            }
        }

        result
    }

    #[instrument(skip(db, all_players, turn), level = Level::DEBUG)]
    pub fn apply_action_results(
        db: &mut Database,
        all_players: &mut AllPlayers,
        turn: &Turn,
        results: &[ActionResult],
    ) -> PendingResults {
        let mut pending = PendingResults::default();

        for result in results.iter() {
            pending.extend(Self::apply_action_result(db, all_players, turn, result));
        }

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        pending
    }

    #[instrument(skip(db, all_players, turn), level = Level::DEBUG)]
    fn apply_action_result(
        db: &mut Database,
        all_players: &mut AllPlayers,
        turn: &Turn,
        result: &ActionResult,
    ) -> PendingResults {
        match result {
            ActionResult::Discard(card) => {
                assert!(card.is_in_location::<InHand>(db));
                card.move_to_graveyard(db);
                PendingResults::default()
            }
            ActionResult::TapPermanent(card_id) => card_id.tap(db),
            ActionResult::PermanentToGraveyard(card_id) => {
                Self::permanent_to_graveyard(db, turn, *card_id)
            }
            ActionResult::AddAbilityToStack {
                source,
                ability,
                targets,
                x_is,
            } => {
                ability.move_to_stack(db, *source, targets.clone());
                if let Some(x) = x_is {
                    source.set_x(db, *x);
                }
                PendingResults::default()
            }
            ActionResult::AddTriggerToStack {
                trigger,
                source,
                targets,
            } => {
                trigger.move_to_stack(db, *source, targets.clone());
                PendingResults::default()
            }
            ActionResult::CloneCreatureNonTargeting { source, target } => {
                if let ActiveTarget::Battlefield { id: target } = target {
                    source.clone_card(db, *target, OnBattlefield::new());
                }
                PendingResults::default()
            }
            ActionResult::AddModifier { modifier } => {
                modifier.activate(db);
                PendingResults::default()
            }
            ActionResult::Mill { count, targets } => {
                let mut pending = PendingResults::default();
                for target in targets {
                    let ActiveTarget::Player { id: target } = target else {
                        unreachable!()
                    };

                    let deck = &mut all_players[*target].deck;
                    for _ in 0..*count {
                        let card_id = deck.draw();
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

                    all_players[target.owner(db)].deck.place_on_top(db, *target);
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
            ActionResult::ReturnFromBattlefieldToLibrary { target } => {
                let ActiveTarget::Battlefield { id: target } = target else {
                    unreachable!()
                };

                all_players[target.owner(db)].deck.place_on_top(db, *target);
                PendingResults::default()
            }
            ActionResult::LoseLife { target, count } => {
                all_players[*target].life_total -= *count as i32;
                PendingResults::default()
            }
            ActionResult::GainMana {
                gain,
                target,
                source,
                restriction,
            } => {
                for mana in gain {
                    all_players[*target]
                        .mana_pool
                        .apply(*mana, *source, *restriction)
                }
                PendingResults::default()
            }
            ActionResult::CreateToken { source, token } => {
                let mut results = PendingResults::default();

                let mut replacements =
                    ReplacementEffectId::watching::<replacing::TokenCreation>(db).into_iter();

                let card = CardId::upload_token(db, source.controller(db).into(), token.clone());
                card.move_to_limbo(db);
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
            ActionResult::DrawCards { target, count } => {
                let _ = all_players[*target].draw(db, *count);
                PendingResults::default()
            }
            ActionResult::AddToBattlefield(card, target) => {
                Battlefield::add_from_stack_or_hand(db, *card, *target)
            }
            ActionResult::StackToGraveyard(card) => {
                if card.is_in_location::<InStack>(db) {
                    return Battlefield::stack_to_graveyard(db, *card);
                }
                PendingResults::default()
            }
            ActionResult::ApplyToBattlefield(modifier) => {
                modifier.activate(db);
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
                    if !source.is_in_location::<OnBattlefield>(db) {
                        return PendingResults::default();
                    }
                }

                Battlefield::exile(db, turn, *source, target, *reason, *duration)
            }
            ActionResult::DamageTarget { quantity, target } => {
                match target {
                    ActiveTarget::Battlefield { id } => {
                        id.mark_damage(db, *quantity);
                    }
                    ActiveTarget::Player { id } => {
                        all_players[*id].life_total -= *quantity as i32;
                    }
                    ActiveTarget::Graveyard { .. }
                    | ActiveTarget::Library { .. }
                    | ActiveTarget::Stack { .. } => unreachable!(),
                }
                PendingResults::default()
            }
            ActionResult::ManifestTopOfLibrary(player) => all_players[*player].manifest(db),
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
            ActionResult::SpellCountered { id } => match id {
                Entry::Card(card) => Battlefield::stack_to_graveyard(db, *card),
                Entry::Ability { .. } | Entry::Trigger { .. } => unreachable!(),
            },
            ActionResult::AddCounters {
                source,
                target,
                count,
                counter,
            } => {
                match count {
                    GainCount::Single => {
                        CounterId::add_counters(db, *target, *counter, 1);
                    }
                    GainCount::Dynamic(dynamic) => match dynamic {
                        DynamicCounter::X => {
                            let x = source.get_x(db);
                            if x > 0 {
                                CounterId::add_counters(db, *target, *counter, x);
                            }
                        }
                        DynamicCounter::LeftBattlefieldThisTurn {
                            controller,
                            restrictions,
                        } => {
                            let cards = CardId::left_battlefield_this_turn(db, turn.turn_count);
                            let x = cards
                                .into_iter()
                                .filter(|card| {
                                    card.passes_restrictions(db, *source, *controller, restrictions)
                                })
                                .count();
                            if x > 0 {
                                CounterId::add_counters(db, *target, *counter, x);
                            }
                        }
                    },
                }

                PendingResults::default()
            }
            ActionResult::RevealCard(card) => {
                card.reveal(db);
                PendingResults::default()
            }
            ActionResult::MoveToHandFromLibrary(card) => {
                card.move_to_hand(db);
                all_players[card.controller(db)].deck.remove(*card);
                PendingResults::default()
            }
            ActionResult::AddToBattlefieldFromLibrary {
                card,
                enters_tapped,
            } => {
                all_players[card.controller(db)].deck.remove(*card);
                Battlefield::add_from_library(db, *card, *enters_tapped)
            }
            ActionResult::Shuffle(owner) => {
                all_players[*owner].deck.shuffle();
                PendingResults::default()
            }
            ActionResult::ApplyAuraToTarget { aura, target } => {
                match target {
                    ActiveTarget::Battlefield { id } => {
                        id.apply_aura(db, *aura);
                    }
                    ActiveTarget::Graveyard { .. } => todo!(),
                    ActiveTarget::Player { .. } => todo!(),
                    _ => unreachable!(),
                };
                PendingResults::default()
            }
            ActionResult::PlayerLoses(player) => {
                all_players[*player].lost = true;
                PendingResults::default()
            }
            ActionResult::CastCard {
                card,
                targets,
                from,
                x_is,
                chosen_modes,
            } => {
                let mut results = PendingResults::default();

                card.move_to_stack(db, targets.clone(), Some(*from), chosen_modes.clone());
                if let Some(x_is) = x_is {
                    card.set_x(db, *x_is)
                };
                card.apply_modifiers_layered(db);
                let cascade = card.cascade(db);
                for _ in 0..cascade {
                    let id = TriggerId::upload(
                        db,
                        &TriggeredAbility {
                            trigger: Trigger {
                                trigger: TriggerSource::Cast,
                                from: triggers::Location::Hand,
                                controller: ControllerRestriction::You,
                                restrictions: Default::default(),
                            },
                            effects: vec![AnyEffect {
                                effect: Effect(&Cascade),
                                threshold: None,
                                oracle_text: Default::default(),
                            }],
                            oracle_text: "Cascade".to_string(),
                        },
                        *card,
                        true,
                    );

                    results.extend(Stack::move_trigger_to_stack(db, id, *card));
                }

                results
            }
            ActionResult::UpdateStackEntries(entries) => {
                for entry in entries.iter() {
                    match entry.ty {
                        Entry::Card(card) => {
                            card.move_to_stack(db, entry.targets.clone(), None, vec![]);
                        }
                        Entry::Ability { in_stack, .. } => {
                            in_stack.update_stack_seq(db);
                        }
                        Entry::Trigger { in_stack, .. } => {
                            in_stack.update_stack_seq(db);
                        }
                    }
                }
                PendingResults::default()
            }
            ActionResult::HandFromBattlefield(card) => Self::permanent_to_hand(db, *card),
            ActionResult::RevealEachTopOfLibrary(source, reveal) => {
                let players = all_players.all_players();
                let revealed = players
                    .into_iter()
                    .filter_map(|player| all_players[player].deck.reveal_top(db))
                    .collect_vec();
                let revealed = revealed
                    .into_iter()
                    .filter(|card| {
                        card.passes_restrictions(
                            db,
                            *source,
                            ControllerRestriction::Any,
                            &reveal.for_each.restrictions,
                        )
                    })
                    .collect_vec();

                let mut results = PendingResults::default();
                if revealed.is_empty() {
                    let controller = source.controller(db);
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
                if let Some(sourced) = card.get_mana_from_sources(db) {
                    if let Some(from_source) = sourced.get(source) {
                        for _ in 0..*from_source {
                            effect.push_pending_behavior(
                                db,
                                *card,
                                card.controller(db),
                                &mut results,
                            );
                        }
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

                let mut replacements =
                    ReplacementEffectId::watching::<replacing::TokenCreation>(db).into_iter();

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
                let owner = card.owner(db);
                all_players[owner].deck.remove(*card);
                all_players[owner].deck.place_on_top(db, *card);
                PendingResults::default()
            }
            ActionResult::SpendMana {
                card,
                mana,
                sources,
                reason,
            } => {
                card.mana_from_source(db, sources);
                let spent = all_players[card.controller(db)].spend_mana(db, mana, sources, *reason);
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
                results.cast_from(CastFrom::Exile);

                while let Some(card) = all_players[*player].deck.exile_top_card(
                    db,
                    *source,
                    Some(ExileReason::Cascade),
                ) {
                    if !card.is_land(db) && card.cost(db).cmc() < *cascading {
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
                results.cast_from(CastFrom::Exile);

                while let Some(card) = all_players[*player].deck.exile_top_card(
                    db,
                    *source,
                    Some(ExileReason::Cascade),
                ) {
                    if !card.is_land(db) && card.cost(db).cmc() < *count {
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

                let player = &mut all_players[*player];
                for card in cards {
                    player.deck.place_on_bottom(db, card);
                }
                PendingResults::default()
            }
            ActionResult::Scry(source, count) => {
                let mut cards = vec![];
                for _ in 0..*count {
                    if let Some(card) = all_players[source.controller(db)].deck.draw() {
                        cards.push(card);
                    } else {
                        break;
                    }
                }

                let mut results = PendingResults::default();
                results.push_choose_scry(source.controller(db), cards);

                results
            }
            ActionResult::GainLife { target, count } => {
                all_players[*target].life_total += *count as i32;
                update_life_gained_this_turn(db, (*target).into(), *count);
                PendingResults::default()
            }
            ActionResult::DeclareAttackers { attackers, targets } => {
                let mut results = PendingResults::default();
                for (attacker, target) in attackers.iter().zip(targets.iter()) {
                    attacker.set_attacking(db, *target);

                    let listeners =
                        TriggerId::active_triggers_of_source::<trigger_source::Attacks>(db);
                    debug!("attack listeners {:?}", listeners);
                    for trigger in listeners {
                        if attacker.passes_restrictions(
                            db,
                            trigger.listener(db),
                            trigger.controller_restriction(db),
                            &trigger.restrictions(db),
                        ) {
                            results.extend(Stack::move_trigger_to_stack(db, trigger, *attacker));
                        }
                    }

                    if attacker.battle_cry(db) {
                        let trigger = TriggerId::upload(
                            db,
                            &TriggeredAbility {
                                trigger: Trigger {
                                    trigger: TriggerSource::Attacks,
                                    from: triggers::Location::Anywhere,
                                    controller: ControllerRestriction::You,
                                    restrictions: vec![],
                                },
                                effects: vec![AnyEffect {
                                    effect: Effect(&BattleCry),
                                    threshold: None,
                                    oracle_text: String::default(),
                                }],
                                oracle_text: "Battle cry".to_string(),
                            },
                            *attacker,
                            true,
                        );

                        results.extend(Stack::move_trigger_to_stack(db, trigger, *attacker));
                    }

                    if !attacker.vigilance(db) {
                        results.extend(attacker.tap(db));
                    }
                }
                debug!(
                    "Set number of attackers to {} in turn {}",
                    attackers.len(),
                    turn.turn_count
                );
                db.insert_resource(NumberOfAttackers {
                    count: attackers.len(),
                    turn: turn.turn_count,
                });
                // TODO declare blockers
                results
            }
            ActionResult::DestroyEach(source, restrictions) => {
                let cards = in_play::cards::<OnBattlefield>(db)
                    .into_iter()
                    .filter(|card| {
                        card.passes_restrictions(
                            db,
                            *source,
                            ControllerRestriction::Any,
                            restrictions,
                        )
                    })
                    .collect_vec();

                let mut results = PendingResults::default();
                for card in cards {
                    results.extend(Battlefield::permanent_to_graveyard(db, turn, card));
                }

                results
            }
            ActionResult::DestroyTarget(target) => {
                let ActiveTarget::Battlefield { id } = target else {
                    unreachable!()
                };

                Battlefield::permanent_to_graveyard(db, turn, *id)
            }
            ActionResult::Explore { target } => {
                let explorer = target.id().unwrap();
                if let Some(card) = all_players[explorer.controller(db)].deck.draw() {
                    card.reveal(db);
                    if card.types_intersect(db, &IndexSet::from([Type::BasicLand, Type::Land])) {
                        card.move_to_hand(db);
                        PendingResults::default()
                    } else {
                        CounterId::add_counters(db, explorer, Counter::P1P1, 1);
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

                for card in id.get_cards::<InGraveyard>(db) {
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
                move_card_to_battlefield(
                    db,
                    target.faceup_face(db),
                    *enters_tapped,
                    &mut results,
                    None,
                );
                if target.is_in_location::<InExile>(db) {
                    complete_add_from_exile(db, target.faceup_face(db), &mut results);
                } else if target.is_in_location::<InGraveyard>(db) {
                    complete_add_from_graveyard(db, target.faceup_face(db), &mut results);
                } else {
                    unreachable!()
                }

                target.move_to_limbo(db);
                results
            }
            ActionResult::Transform { target } => {
                target.transform(db);
                target.move_to_limbo(db);
                target.faceup_face(db).move_to_battlefield(db);

                PendingResults::default()
            }
        }
    }

    pub fn permanent_to_hand(db: &mut Database, target: CardId) -> PendingResults {
        target.move_to_hand(db);
        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        PendingResults::default()
    }

    pub fn permanent_to_graveyard(
        db: &mut Database,
        turn: &Turn,
        target: CardId,
    ) -> PendingResults {
        let mut pending = PendingResults::default();

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(
                trigger.location_from(db),
                triggers::Location::Anywhere | triggers::Location::Battlefield
            ) {
                let restrictions = trigger.restrictions(db);
                if target.passes_restrictions(
                    db,
                    trigger.listener(db),
                    trigger.controller_restriction(db),
                    &restrictions,
                ) {
                    let listener = trigger.listener(db);
                    pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                }
            }
        }

        pending.extend(Self::leave_battlefield(db, turn, target));
        target.move_to_graveyard(db);

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        pending
    }

    pub fn library_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(
                trigger.location_from(db),
                triggers::Location::Anywhere | triggers::Location::Library
            ) {
                let restrictions = trigger.restrictions(db);
                if target.passes_restrictions(
                    db,
                    trigger.listener(db),
                    trigger.controller_restriction(db),
                    &restrictions,
                ) {
                    let listener = trigger.listener(db);
                    pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                }
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    pub fn leave_battlefield(db: &mut Database, turn: &Turn, target: CardId) -> PendingResults {
        let mut results = PendingResults::default();

        for card in in_play::cards::<InExile>(db)
            .iter()
            .filter(|card| {
                card.exile_source(db) == target && card.until_source_leaves_battlefield(db)
            })
            .collect_vec()
        {
            results.extend(Battlefield::add_from_exile(db, *card, false, None));
        }

        target.left_battlefield(db, turn.turn_count);

        results
    }

    pub fn stack_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(trigger.location_from(db), triggers::Location::Anywhere) {
                let restrictions = trigger.restrictions(db);
                if target.passes_restrictions(
                    db,
                    trigger.listener(db),
                    trigger.controller_restriction(db),
                    &restrictions,
                ) {
                    let listener = trigger.listener(db);
                    pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                }
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    pub fn exile(
        db: &mut Database,
        turn: &Turn,
        source: CardId,
        target: CardId,
        reason: Option<ExileReason>,
        duration: EffectDuration,
    ) -> PendingResults {
        target.move_to_exile(db, source, reason, duration);

        let mut results = PendingResults::default();
        if let Some(ExileReason::Craft) = reason {
            for trigger in
                TriggerId::active_triggers_of_source::<trigger_source::ExiledDuringCraft>(db)
            {
                if matches!(
                    trigger.location_from(db),
                    triggers::Location::Anywhere | triggers::Location::Battlefield
                ) {
                    let restrictions = trigger.restrictions(db);
                    if source.passes_restrictions(
                        db,
                        trigger.listener(db),
                        trigger.controller_restriction(db),
                        &restrictions,
                    ) {
                        let listener = trigger.listener(db);
                        results.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                    }
                }
            }
        }

        results.extend(Self::leave_battlefield(db, turn, target));
        results
    }
}

#[instrument(skip(db, modifiers, results))]
pub fn create_token_copy_with_replacements(
    db: &mut Database,
    source: CardId,
    copying: CardId,
    modifiers: &[ModifyBattlefield],
    replacements: &mut IntoIter<ReplacementEffectId>,
    results: &mut PendingResults,
) {
    let mut replaced = false;
    if replacements.len() > 0 {
        while let Some(replacement) = replacements.next() {
            let replacement_source = replacement.source(db);
            let controller_restriction = replacement.controller_restriction(db);
            let restrictions = replacement.restrictions(db);
            if !source.passes_restrictions(
                db,
                replacement_source,
                controller_restriction,
                &source.restrictions(db),
            ) || !copying.passes_restrictions(db, source, controller_restriction, &restrictions)
            {
                continue;
            }

            debug!("Replacing token creation");

            replaced = true;
            for effect in replacement.effects(db) {
                effect
                    .into_effect(db, replacement_source.controller(db))
                    .replace_token_creation(db, source, replacements, copying, modifiers, results);
            }
            break;
        }
    }

    if !replaced {
        debug!("Creating token");
        let token = copying.token_copy_of(db, source.controller(db).into());
        for modifier in modifiers.iter() {
            let modifier = ModifierId::upload_temporary_modifier(
                db,
                token,
                &BattlefieldModifier {
                    modifier: modifier.clone(),
                    controller: ControllerRestriction::Any,
                    duration: EffectDuration::UntilSourceLeavesBattlefield,
                    restrictions: vec![],
                },
            );

            token.apply_modifier(db, modifier);
        }
        results.extend(Battlefield::add_from_stack_or_hand(db, token, None));
    }
}

pub fn compute_deck_targets(
    db: &mut Database,
    player: Controller,
    restrictions: &[Restriction],
) -> Vec<CardId> {
    let mut results = vec![];

    for card in player.get_cards_in::<InLibrary>(db) {
        if !card.passes_restrictions(db, card, ControllerRestriction::You, restrictions) {
            continue;
        }

        results.push(card);
    }

    results
}

pub fn compute_graveyard_targets(
    db: &mut Database,
    controller: ControllerRestriction,
    source_card: CardId,
    restrictions: &[Restriction],
) -> Vec<CardId> {
    let targets = match controller {
        ControllerRestriction::Any => AllPlayers::all_players_in_db(db),
        ControllerRestriction::You => HashSet::from([source_card.controller(db).into()]),
        ControllerRestriction::Opponent => {
            let mut all = AllPlayers::all_players_in_db(db);
            all.remove(&source_card.controller(db).into());
            all
        }
    };
    let mut target_cards = vec![];

    for target in targets.into_iter() {
        let cards_in_graveyard = target.get_cards::<InGraveyard>(db);
        for card in cards_in_graveyard {
            if !card.passes_restrictions(db, source_card, controller, &source_card.restrictions(db))
                || !card.passes_restrictions(db, source_card, controller, restrictions)
            {
                continue;
            }

            target_cards.push(card);
        }
    }

    target_cards
}

fn complete_add_from_library(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for trigger in TriggerId::active_triggers_of_source::<trigger_source::EntersTheBattlefield>(db)
    {
        if matches!(
            trigger.location_from(db),
            triggers::Location::Anywhere | triggers::Location::Library
        ) {
            let restrictions = trigger.restrictions(db);
            if source_card_id.passes_restrictions(
                db,
                trigger.listener(db),
                trigger.controller_restriction(db),
                &restrictions,
            ) {
                let listener = trigger.listener(db);
                results.extend(Stack::move_trigger_to_stack(db, trigger, listener));
            }
        }
    }

    for card in all_cards(db) {
        card.apply_modifiers_layered(db);
    }
}

fn complete_add_from_exile(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for trigger in TriggerId::active_triggers_of_source::<trigger_source::EntersTheBattlefield>(db)
    {
        if matches!(trigger.location_from(db), triggers::Location::Anywhere) {
            let restrictions = trigger.restrictions(db);
            if source_card_id.passes_restrictions(
                db,
                trigger.listener(db),
                trigger.controller_restriction(db),
                &restrictions,
            ) {
                let listener = trigger.listener(db);
                results.extend(Stack::move_trigger_to_stack(db, trigger, listener));
            }
        }
    }

    for card in all_cards(db) {
        card.apply_modifiers_layered(db);
    }
}

fn complete_add_from_graveyard(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for trigger in TriggerId::active_triggers_of_source::<trigger_source::EntersTheBattlefield>(db)
    {
        if matches!(trigger.location_from(db), triggers::Location::Anywhere) {
            let restrictions = trigger.restrictions(db);
            if source_card_id.passes_restrictions(
                db,
                trigger.listener(db),
                trigger.controller_restriction(db),
                &restrictions,
            ) {
                let listener = trigger.listener(db);
                results.extend(Stack::move_trigger_to_stack(db, trigger, listener));
            }
        }
    }

    for card in all_cards(db) {
        card.apply_modifiers_layered(db);
    }
}

fn complete_add_from_stack_or_hand(
    db: &mut Database,
    source_card_id: CardId,
    results: &mut PendingResults,
) {
    for trigger in TriggerId::active_triggers_of_source::<trigger_source::EntersTheBattlefield>(db)
    {
        if matches!(trigger.location_from(db), triggers::Location::Anywhere) {
            let restrictions = trigger.restrictions(db);
            if source_card_id.passes_restrictions(
                db,
                trigger.listener(db),
                trigger.controller_restriction(db),
                &restrictions,
            ) {
                let listener = trigger.listener(db);
                results.extend(Stack::move_trigger_to_stack(db, trigger, listener));
            }
        }
    }

    for card in all_cards(db) {
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
    if let Some(aura) = source_card_id.aura(db) {
        target.unwrap().apply_aura(db, aura);
    }
    for ability in source_card_id.static_abilities(db) {
        match ability {
            StaticAbility::BattlefieldModifier(modifier) => {
                let modifier = ModifierId::upload_temporary_modifier(db, source_card_id, &modifier);
                results.push_settled(ActionResult::AddModifier { modifier })
            }
            _ => {}
        }
    }
    for ability in source_card_id.etb_abilities(db) {
        results.extend(Stack::move_etb_ability_to_stack(
            db,
            ability,
            source_card_id,
        ));
    }

    let must_enter_tapped =
        Battlefield::static_abilities(db)
            .iter()
            .any(|(ability, controller)| match ability {
                StaticAbility::ForceEtbTapped(ForceEtbTapped {
                    controller: controller_restriction,
                    types,
                }) => {
                    match controller_restriction {
                        ControllerRestriction::Any => {}
                        ControllerRestriction::You => {
                            if *controller != source_card_id.controller(db) {
                                return false;
                            }
                        }
                        ControllerRestriction::Opponent => {
                            if *controller == source_card_id.controller(db) {
                                return false;
                            }
                        }
                    }

                    source_card_id.types_intersect(db, types)
                }
                _ => false,
            });

    if must_enter_tapped || source_card_id.etb_tapped(db) || enters_tapped {
        results.extend(source_card_id.tap(db));
    }
    source_card_id.move_to_battlefield(db);
}
