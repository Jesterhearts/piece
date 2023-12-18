mod pending_results;

use std::collections::HashSet;

use bevy_ecs::{entity::Entity, query::With};

use itertools::Itertools;

use crate::{
    abilities::{Ability, GainMana, StaticAbility},
    card::Color,
    controller::ControllerRestriction,
    cost::{AdditionalCost, PayLife},
    effects::{
        effect_duration::UntilEndOfTurn, replacing, BattlefieldModifier, Counter, Effect,
        EffectDuration, RevealEachTopOfLibrary, Token,
    },
    in_play::{
        all_cards, cards, AbilityId, Active, AuraId, CardId, CounterId, Database, InGraveyard,
        InLibrary, InStack, ModifierId, OnBattlefield, ReplacementEffectId, TriggerId,
    },
    mana::Mana,
    player::{AllPlayers, Controller, Owner},
    stack::{ActiveTarget, Entry, Stack, StackEntry},
    targets::Restriction,
    triggers::{self, trigger_source},
    turns::Turn,
    types::Type,
};

pub use pending_results::{
    ChooseTargets, EffectOrAura, PayCost, PendingResults, ResolutionResult, SacrificePermanent,
    Source, SpendMana,
};

#[must_use]
#[derive(Debug)]
pub enum PartialAddToBattlefieldResult {
    NeedsResolution(PendingResults),
    Continue(PendingResults),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    UpdateStackEntries(Vec<StackEntry>),
    PlayerLoses(Owner),
    RevealCard(CardId),
    MoveToHandFromLibrary(CardId),
    Shuffle(Owner),
    AddToBattlefield(CardId, Option<CardId>),
    AddToBattlefieldSkipReplacementEffects(CardId, Option<CardId>),
    AddToBattlefieldSkipReplacementEffectsFromLibrary {
        card: CardId,
        enters_tapped: bool,
    },
    AddToBattlefieldFromLibrary {
        card: CardId,
        enters_tapped: bool,
    },
    StackToGraveyard(CardId),
    ApplyToBattlefield(ModifierId),
    ApplyAuraToTarget {
        aura: AuraId,
        target: ActiveTarget,
    },
    ExileTarget(ActiveTarget),
    DamageTarget {
        quantity: usize,
        target: ActiveTarget,
    },
    ManifestTopOfLibrary(Controller),
    ModifyCreatures {
        targets: Vec<ActiveTarget>,
        modifier: ModifierId,
    },
    SpellCountered {
        id: Entry,
    },
    AddCounters {
        target: CardId,
        counter: Counter,
        count: usize,
    },
    TapPermanent(CardId),
    PermanentToGraveyard(CardId),
    AddAbilityToStack {
        source: CardId,
        ability: AbilityId,
        targets: Vec<Vec<ActiveTarget>>,
    },
    AddTriggerToStack {
        trigger: TriggerId,
        source: CardId,
        targets: Vec<Vec<ActiveTarget>>,
    },
    CloneCreatureNonTargeting {
        source: CardId,
        target: Option<ActiveTarget>,
    },
    AddModifier {
        modifier: ModifierId,
    },
    Mill {
        count: usize,
        targets: Vec<ActiveTarget>,
    },
    ReturnFromGraveyardToLibrary {
        targets: Vec<ActiveTarget>,
    },
    ReturnFromGraveyardToBattlefield {
        targets: Vec<ActiveTarget>,
    },
    LoseLife {
        target: Controller,
        count: usize,
    },
    GainMana {
        gain: Vec<Mana>,
        target: Controller,
    },
    CreateToken {
        source: Controller,
        token: Token,
    },
    DrawCards {
        target: Controller,
        count: usize,
    },
    AddCardToStack {
        card: CardId,
        targets: Vec<Vec<ActiveTarget>>,
    },
    HandFromBattlefield(CardId),
    RevealEachTopOfLibrary(CardId, RevealEachTopOfLibrary),
    CreateTokenCopyOf {
        target: CardId,
        modifiers: Vec<crate::effects::ModifyBattlefield>,
        controller: Controller,
    },
    MoveFromLibraryToTopOfLibrary(CardId),
    SpendMana(Controller, Vec<Mana>),
    Untap(CardId),
    ReturnFromBattlefieldToLibrary {
        target: ActiveTarget,
    },
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

    fn start_adding_to_battlefield(
        db: &mut Database,
        source_card_id: CardId,
        enters_tapped: bool,
        target: Option<CardId>,
        construct_skip_replacement: impl FnOnce(CardId, bool) -> ActionResult,
    ) -> PartialAddToBattlefieldResult {
        let mut results = PendingResults::new(Source::Card(source_card_id));

        ReplacementEffectId::activate_all_for_card(db, source_card_id);
        for replacement in ReplacementEffectId::watching::<replacing::Etb>(db) {
            let source = replacement.source(db);
            if source != source_card_id {
                continue;
            }

            let restrictions = replacement.restrictions(db);
            if !source.passes_restrictions(db, source, ControllerRestriction::Any, &restrictions) {
                continue;
            }

            let controller = replacement.source(db).controller(db);
            for effect in replacement.effects(db) {
                let effect = effect.into_effect(db, controller);
                Self::push_effect_results(db, source, controller, effect, &mut results);
            }

            if !results.only_immediate_results() {
                source_card_id.apply_modifiers_layered(db);
                results.push_settled(construct_skip_replacement(source_card_id, enters_tapped));
                return PartialAddToBattlefieldResult::NeedsResolution(results);
            }
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

        for card in player.get_cards::<InLibrary>(db) {
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
        types: &HashSet<Type>,
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
        let cards = player.get_cards::<OnBattlefield>(db);

        let mut colors = HashSet::default();
        for card in cards {
            let card_colors = card.colors(db);
            colors.extend(card_colors)
        }

        colors
    }

    pub fn untap(db: &mut Database, player: Owner) {
        let cards = player.get_cards::<OnBattlefield>(db);
        for card in cards {
            card.untap(db);
        }
    }

    pub fn end_turn(db: &mut Database) {
        let cards = cards::<OnBattlefield>(db);
        for card in cards {
            card.clear_damage(db);
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
    }

    pub fn check_sba(db: &mut Database) -> Vec<ActionResult> {
        let mut result = vec![];
        for card_id in cards::<OnBattlefield>(db) {
            let toughness = card_id.toughness(db);

            if toughness.is_some() && (toughness.unwrap() - card_id.marked_damage(db)) <= 0 {
                result.push(ActionResult::PermanentToGraveyard(card_id));
            }

            let aura = card_id.aura(db);
            if aura.is_some() && !aura.unwrap().is_attached(db) {
                result.push(ActionResult::PermanentToGraveyard(card_id));
            }
        }

        result
    }

    pub fn activate_ability(
        db: &mut Database,
        all_players: &mut AllPlayers,
        turn: &Turn,
        card: CardId,
        index: usize,
    ) -> PendingResults {
        if Stack::split_second(db) {
            return PendingResults::default();
        }

        let ability_id = card.activated_abilities(db)[index];
        let ability = ability_id.ability(db);

        if !ability_id.can_be_activated(db, all_players, turn) {
            return PendingResults::default();
        }

        let mut results = PendingResults::new(pending_results::Source::Ability(ability_id));
        if let Some(cost) = ability.cost() {
            if cost.tap {
                if card.tapped(db) {
                    unreachable!()
                }

                results.push_settled(ActionResult::TapPermanent(card));
            }

            for cost in cost.additional_cost.iter() {
                match cost {
                    AdditionalCost::SacrificeThis => {
                        if !card.can_be_sacrificed(db) {
                            unreachable!()
                        }

                        results.push_settled(ActionResult::PermanentToGraveyard(card));
                    }
                    AdditionalCost::PayLife(PayLife { count }) => {
                        results.push_settled(ActionResult::LoseLife {
                            target: card.controller(db),
                            count: *count,
                        })
                    }
                    AdditionalCost::SacrificePermanent(restrictions) => {
                        results.push_pay_costs(PayCost::SacrificePermanent(
                            SacrificePermanent::new(restrictions.clone()),
                        ));
                    }
                }
            }

            results.push_pay_costs(PayCost::SpendMana(SpendMana::new(cost.mana_cost.clone())));
        }

        if let Ability::Mana(gain) = ability {
            if let GainMana::Choice { .. } = gain.gain {
                results.push_choose_mode();
            }
        } else {
            results.add_to_stack();
            let controller = card.controller(db);

            let creatures = Self::creatures(db);
            for effect in ability.into_effects() {
                let effect = effect.into_effect(db, controller);
                let targets = card.targets_for_effect(db, controller, &effect, &creatures);

                results
                    .push_choose_targets(ChooseTargets::new(EffectOrAura::Effect(effect), targets));
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

    pub fn apply_action_results(
        db: &mut Database,
        all_players: &mut AllPlayers,
        results: &[ActionResult],
    ) -> PendingResults {
        let mut pending = PendingResults::default();

        for result in results.iter() {
            pending.extend(Self::apply_action_result(db, all_players, result));
        }

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        pending
    }

    fn apply_action_result(
        db: &mut Database,
        all_players: &mut AllPlayers,
        result: &ActionResult,
    ) -> PendingResults {
        match result {
            ActionResult::TapPermanent(card_id) => {
                card_id.tap(db);
            }
            ActionResult::PermanentToGraveyard(card_id) => {
                return Self::permanent_to_graveyard(db, *card_id);
            }
            ActionResult::AddAbilityToStack {
                source,
                ability,
                targets,
            } => {
                ability.move_to_stack(db, *source, targets.clone());
            }
            ActionResult::AddTriggerToStack {
                trigger,
                source,
                targets,
            } => {
                trigger.move_to_stack(db, *source, targets.clone());
            }
            ActionResult::CloneCreatureNonTargeting { source, target } => {
                if let Some(ActiveTarget::Battlefield { id: target }) = target {
                    source.clone_card(db, *target, OnBattlefield::new());
                }
            }
            ActionResult::AddModifier { modifier } => {
                modifier.activate(db);
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

                return pending;
            }
            ActionResult::ReturnFromGraveyardToLibrary { targets } => {
                for target in targets {
                    let ActiveTarget::Graveyard { id: target } = target else {
                        unreachable!()
                    };

                    all_players[target.owner(db)].deck.place_on_top(db, *target);
                }
            }
            ActionResult::ReturnFromGraveyardToBattlefield { targets } => {
                let mut pending = PendingResults::default();
                for target in targets {
                    let ActiveTarget::Graveyard { id: target } = target else {
                        unreachable!()
                    };
                    pending.extend(Self::add_from_stack_or_hand(db, *target, None));
                }

                return pending;
            }
            ActionResult::ReturnFromBattlefieldToLibrary { target } => {
                let ActiveTarget::Battlefield { id: target } = target else {
                    unreachable!()
                };

                all_players[target.owner(db)].deck.place_on_top(db, *target);
            }
            ActionResult::LoseLife { target, count } => {
                all_players[*target].life_total -= *count as i32;
            }
            ActionResult::GainMana { gain, target } => {
                for mana in gain {
                    all_players[*target].mana_pool.apply(*mana)
                }
            }
            ActionResult::CreateToken { source, token } => {
                let card = CardId::upload_token(db, (*source).into(), token.clone());
                return Battlefield::add_from_stack_or_hand(db, card, None);
            }
            ActionResult::DrawCards { target, count } => {
                let _ = all_players[*target].draw(db, *count);
            }
            ActionResult::AddToBattlefield(card, target) => {
                return Battlefield::add_from_stack_or_hand(db, *card, *target);
            }
            ActionResult::StackToGraveyard(card) => {
                if card.is_in_location::<InStack>(db) {
                    return Battlefield::stack_to_graveyard(db, *card);
                }
            }
            ActionResult::ApplyToBattlefield(modifier) => {
                modifier.activate(db);
            }
            ActionResult::ExileTarget(target) => {
                let ActiveTarget::Battlefield { id: target } = target else {
                    unreachable!()
                };
                return Battlefield::exile(db, *target);
            }
            ActionResult::DamageTarget { quantity, target } => match target {
                ActiveTarget::Battlefield { id } => {
                    id.mark_damage(db, *quantity);
                }
                ActiveTarget::Player { id } => {
                    all_players[*id].life_total -= *quantity as i32;
                }
                ActiveTarget::Graveyard { .. }
                | ActiveTarget::Library { .. }
                | ActiveTarget::Stack { .. } => unreachable!(),
            },
            ActionResult::ManifestTopOfLibrary(player) => {
                return all_players[*player].manifest(db);
            }
            ActionResult::ModifyCreatures { targets, modifier } => {
                for target in targets {
                    let ActiveTarget::Battlefield { id: target } = target else {
                        unreachable!()
                    };
                    target.apply_modifier(db, *modifier);
                }
            }
            ActionResult::SpellCountered { id } => match id {
                Entry::Card(card) => {
                    return Battlefield::stack_to_graveyard(db, *card);
                }
                Entry::Ability { .. } | Entry::Trigger { .. } => unreachable!(),
            },
            ActionResult::AddCounters {
                target,
                counter,
                count,
            } => {
                CounterId::add_counters(db, *target, *counter, *count);
            }
            ActionResult::RevealCard(card) => {
                card.reveal(db);
            }
            ActionResult::MoveToHandFromLibrary(card) => {
                card.move_to_hand(db);
                all_players[card.controller(db)].deck.remove(*card);
            }
            ActionResult::AddToBattlefieldFromLibrary {
                card,
                enters_tapped,
            } => {
                all_players[card.controller(db)].deck.remove(*card);
                return Battlefield::add_from_library(db, *card, *enters_tapped);
            }
            ActionResult::Shuffle(owner) => {
                all_players[*owner].deck.shuffle();
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
            }
            ActionResult::PlayerLoses(player) => {
                all_players[*player].lost = true;
            }
            ActionResult::AddCardToStack { card, targets } => {
                card.move_to_stack(db, targets.clone());
            }
            ActionResult::UpdateStackEntries(entries) => {
                for entry in entries.iter() {
                    match entry.ty {
                        Entry::Card(card) => {
                            card.move_to_stack(db, entry.targets.clone());
                        }
                        Entry::Ability { in_stack, .. } => {
                            in_stack.update_stack_seq(db);
                        }
                        Entry::Trigger { in_stack, .. } => {
                            in_stack.update_stack_seq(db);
                        }
                    }
                }
            }
            ActionResult::HandFromBattlefield(card) => {
                return Self::permanent_to_hand(db, *card);
            }
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
                        Self::push_effect_results(
                            db,
                            *source,
                            controller,
                            effect.clone(),
                            &mut results,
                        )
                    }
                } else {
                    for target in revealed {
                        for effect in reveal.for_each.effects.iter() {
                            Self::push_effect_results_with_target_from_top_of_library(
                                db,
                                *source,
                                effect,
                                target,
                                &mut results,
                            );
                        }
                    }
                }

                return results;
            }
            ActionResult::CreateTokenCopyOf {
                target,
                modifiers,
                controller,
            } => {
                let token = target.token_copy_of(db, (*controller).into());

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
            }
            ActionResult::MoveFromLibraryToTopOfLibrary(card) => {
                let owner = card.owner(db);
                all_players[owner].deck.remove(*card);
                all_players[owner].deck.place_on_top(db, *card);
            }
            ActionResult::SpendMana(controller, mana) => {
                let spent = all_players[*controller].spend_mana(mana);
                assert!(
                    spent,
                    "Should have validated mana could be spent before spending."
                );
            }
            ActionResult::AddToBattlefieldSkipReplacementEffects(card, target) => {
                let mut results = PendingResults::default();
                move_card_to_battlefield(db, *card, false, &mut results, *target);
                complete_add_from_stack_or_hand(db, *card, &mut results);
                return results;
            }
            ActionResult::AddToBattlefieldSkipReplacementEffectsFromLibrary {
                card,
                enters_tapped,
            } => {
                let mut results = PendingResults::default();
                move_card_to_battlefield(db, *card, *enters_tapped, &mut results, None);
                complete_add_from_library(db, *card, &mut results);
                return results;
            }
            ActionResult::Untap(target) => {
                target.untap(db);
            }
        }

        PendingResults::default()
    }

    pub fn permanent_to_hand(db: &mut Database, target: CardId) -> PendingResults {
        target.move_to_hand(db);
        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        PendingResults::default()
    }

    pub fn permanent_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(
                trigger.location_from(db),
                triggers::Location::Anywhere | triggers::Location::Battlefield
            ) {
                let for_types = trigger.for_types(db);
                if target.types_intersect(db, &for_types) {
                    for listener in trigger.listeners(db) {
                        pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                    }
                }
            }
        }

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
                let for_types = trigger.for_types(db);
                if target.types_intersect(db, &for_types) {
                    for listener in trigger.listeners(db) {
                        pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                    }
                }
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    pub fn stack_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
        let mut pending = PendingResults::default();

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(trigger.location_from(db), triggers::Location::Anywhere) {
                let for_types = trigger.for_types(db);
                if target.types_intersect(db, &for_types) {
                    for listener in trigger.listeners(db) {
                        pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                    }
                }
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    pub fn exile(db: &mut Database, target: CardId) -> PendingResults {
        target.move_to_exile(db);

        PendingResults::default()
    }

    fn push_effect_results(
        db: &mut Database,
        source: CardId,
        controller: Controller,
        effect: Effect,
        results: &mut PendingResults,
    ) {
        match effect {
            Effect::BattlefieldModifier(modifier) => {
                results.push_settled(ActionResult::AddModifier {
                    modifier: ModifierId::upload_temporary_modifier(db, source, &modifier),
                });
            }
            Effect::ControllerDrawCards(count) => {
                results.push_settled(ActionResult::DrawCards {
                    target: controller,
                    count,
                });
            }
            Effect::ControllerLosesLife(count) => {
                results.push_settled(ActionResult::LoseLife {
                    target: controller,
                    count,
                });
            }
            Effect::CreateToken(token) => {
                results.push_settled(ActionResult::CreateToken {
                    source: controller,
                    token,
                });
            }
            Effect::GainCounter(counter) => {
                results.push_settled(ActionResult::AddCounters {
                    target: source,
                    counter,
                    count: 1,
                });
            }
            Effect::ReturnSelfToHand => {
                results.push_settled(ActionResult::HandFromBattlefield(source))
            }
            Effect::RevealEachTopOfLibrary(reveal) => {
                results.push_settled(ActionResult::RevealEachTopOfLibrary(source, reveal));
            }
            Effect::UntapThis => {
                results.push_settled(ActionResult::Untap(source));
            }
            Effect::CopyOfAnyCreatureNonTargeting
            | Effect::TutorLibrary(_)
            | Effect::CounterSpell { .. }
            | Effect::DealDamage(_)
            | Effect::Equip(_)
            | Effect::ExileTargetCreature
            | Effect::ExileTargetCreatureManifestTopOfLibrary
            | Effect::Mill(_)
            | Effect::ModifyCreature(_)
            | Effect::ReturnFromGraveyardToBattlefield(_)
            | Effect::ReturnFromGraveyardToLibrary(_)
            | Effect::CreateTokenCopy { .. }
            | Effect::TargetToTopOfLibrary { .. } => {
                let creatures = Self::creatures(db);
                let valid_targets = source.targets_for_effect(db, controller, &effect, &creatures);
                results.push_choose_targets(ChooseTargets::new(
                    EffectOrAura::Effect(effect),
                    valid_targets,
                ));
            }
        }
    }

    fn push_effect_results_with_target_from_top_of_library(
        db: &mut Database,
        source: CardId,
        effect: &Effect,
        target: CardId,
        results: &mut PendingResults,
    ) {
        match effect {
            Effect::ControllerDrawCards(count) => {
                results.push_settled(ActionResult::DrawCards {
                    target: target.controller(db),
                    count: *count,
                });
            }
            Effect::ControllerLosesLife(count) => {
                results.push_settled(ActionResult::LoseLife {
                    target: target.controller(db),
                    count: *count,
                });
            }
            Effect::CreateToken(token) => {
                results.push_settled(ActionResult::CreateToken {
                    source: target.controller(db),
                    token: token.clone(),
                });
            }
            Effect::GainCounter(counter) => {
                results.push_settled(ActionResult::AddCounters {
                    target: source,
                    counter: *counter,
                    count: 1,
                });
            }
            Effect::CreateTokenCopy { modifiers } => {
                results.push_settled(ActionResult::CreateTokenCopyOf {
                    target,
                    controller: source.controller(db),
                    modifiers: modifiers.clone(),
                })
            }
            &Effect::TutorLibrary(_)
            | Effect::CounterSpell { .. }
            | Effect::DealDamage(_)
            | Effect::Equip(_)
            | Effect::ExileTargetCreature
            | Effect::ExileTargetCreatureManifestTopOfLibrary
            | Effect::Mill(_)
            | Effect::ModifyCreature(_)
            | Effect::ReturnFromGraveyardToBattlefield(_)
            | Effect::ReturnFromGraveyardToLibrary(_)
            | Effect::BattlefieldModifier(_)
            | Effect::CopyOfAnyCreatureNonTargeting
            | Effect::RevealEachTopOfLibrary(_)
            | Effect::UntapThis
            | Effect::ReturnSelfToHand
            | Effect::TargetToTopOfLibrary { .. } => {
                unreachable!()
            }
        }
    }
}

pub fn compute_deck_targets(
    db: &mut Database,
    player: Controller,
    restrictions: &[Restriction],
) -> Vec<CardId> {
    let mut results = vec![];

    for card in player.get_cards::<InLibrary>(db) {
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
    types: &HashSet<Type>,
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
            let for_types = trigger.for_types(db);
            if source_card_id.types_intersect(db, &for_types) {
                for listener in trigger.listeners(db) {
                    results.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                }
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
            let for_types = trigger.for_types(db);
            if source_card_id.types_intersect(db, &for_types) {
                for listener in trigger.listeners(db) {
                    results.extend(Stack::move_trigger_to_stack(db, trigger, listener));
                }
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
            StaticAbility::GreenCannotBeCountered { .. } => {}
            StaticAbility::BattlefieldModifier(modifier) => {
                let modifier = ModifierId::upload_temporary_modifier(db, source_card_id, &modifier);
                results.push_settled(ActionResult::AddModifier { modifier })
            }
            StaticAbility::ExtraLandsPerTurn(_) => {}
        }
    }
    for ability in source_card_id.etb_abilities(db) {
        results.extend(Stack::move_etb_ability_to_stack(
            db,
            ability,
            source_card_id,
        ));
    }
    if source_card_id.etb_tapped(db) || enters_tapped {
        source_card_id.tap(db);
    }
    source_card_id.move_to_battlefield(db);
}
