mod pending_results;

use std::collections::HashSet;

use bevy_ecs::{entity::Entity, query::With};

use indexmap::IndexSet;
use itertools::Itertools;
use rand::{seq::SliceRandom, thread_rng};

use crate::{
    abilities::{Ability, ForceEtbTapped, GainMana, StaticAbility, TriggeredAbility},
    card::Color,
    controller::ControllerRestriction,
    cost::{AdditionalCost, PayLife},
    effects::{
        effect_duration::UntilEndOfTurn, replacing, AnyEffect, BattlefieldModifier, DestroyEach,
        DynamicCounter, Effect, EffectDuration, ForEachManaOfSource, GainCounter,
        RevealEachTopOfLibrary, Token,
    },
    in_play::{
        self, all_cards, cards, AbilityId, Active, AuraId, CardId, CastFrom, CounterId, Database,
        ExileReason, InGraveyard, InLibrary, InStack, ModifierId, OnBattlefield,
        ReplacementEffectId, TriggerId,
    },
    mana::Mana,
    player::{mana_pool::ManaSource, AllPlayers, Controller, Owner},
    stack::{ActiveTarget, Entry, Stack, StackEntry},
    targets::Restriction,
    triggers::{self, trigger_source, Trigger, TriggerSource},
    turns::Turn,
    types::Type,
};

pub use pending_results::{
    ChooseTargets, PayCost, PendingResults, ResolutionResult, SacrificePermanent, Source,
    SpendMana, TapPermanent, TargetSource,
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
        source: CardId,
        target: CardId,
        counter: GainCounter,
    },
    TapPermanent(CardId),
    PermanentToGraveyard(CardId),
    AddAbilityToStack {
        source: CardId,
        ability: AbilityId,
        targets: Vec<Vec<ActiveTarget>>,
        x_is: Option<usize>,
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
        source: Option<ManaSource>,
    },
    CreateToken {
        source: Controller,
        token: Box<Token>,
    },
    DrawCards {
        target: Controller,
        count: usize,
    },
    CastCard {
        card: CardId,
        targets: Vec<Vec<ActiveTarget>>,
        from: CastFrom,
        x_is: Option<usize>,
    },
    HandFromBattlefield(CardId),
    RevealEachTopOfLibrary(CardId, RevealEachTopOfLibrary),
    CreateTokenCopyOf {
        target: CardId,
        modifiers: Vec<crate::effects::ModifyBattlefield>,
        controller: Controller,
    },
    MoveFromLibraryToTopOfLibrary(CardId),
    SpendMana {
        card: CardId,
        mana: Vec<Mana>,
        sources: Vec<Option<ManaSource>>,
    },
    Untap(CardId),
    ReturnFromBattlefieldToLibrary {
        target: ActiveTarget,
    },
    Cascade {
        cascading: usize,
        player: Controller,
    },
    CascadeExileToBottomOfLibrary(Controller),
    Scry(CardId, usize),
    Discover {
        count: usize,
        player: Controller,
    },
    ForEachManaOfSource {
        card: CardId,
        source: ManaSource,
        effect: Box<Effect>,
    },
    GainLife {
        target: crate::player::Controller,
        count: usize,
    },
    Craft {
        transforming: CardId,
        targets: Vec<ActiveTarget>,
    },
    DeclareAttackers {
        attackers: Vec<CardId>,
        targets: Vec<Owner>,
    },
    DestroyEach(CardId, Vec<Restriction>),
    DestroyTarget(ActiveTarget),
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
                            SacrificePermanent::new(restrictions.clone()),
                        ));
                    }
                    AdditionalCost::TapPermanent(restrictions) => {
                        results.push_pay_costs(PayCost::TapPermanent(TapPermanent::new(
                            restrictions.clone(),
                        )));
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
            results.add_ability_to_stack();
            let controller = card.controller(db);

            for effect in ability.into_effects() {
                let effect = effect.into_effect(db, controller);
                let targets = card.targets_for_effect(
                    db,
                    controller,
                    &effect,
                    results.all_currently_targeted(),
                );

                if effect.needs_targets() > targets.len() {
                    return PendingResults::default();
                }

                if !targets.is_empty() {
                    results.push_choose_targets(ChooseTargets::new(
                        TargetSource::Effect(effect),
                        targets,
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
            ActionResult::TapPermanent(card_id) => card_id.tap(db),
            ActionResult::PermanentToGraveyard(card_id) => {
                Self::permanent_to_graveyard(db, *card_id)
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
                if let Some(ActiveTarget::Battlefield { id: target }) = target {
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
            } => {
                for mana in gain {
                    all_players[*target].mana_pool.apply(*mana, *source)
                }
                PendingResults::default()
            }
            ActionResult::CreateToken { source, token } => {
                let card = CardId::upload_token(db, (*source).into(), (**token).clone());
                Battlefield::add_from_stack_or_hand(db, card, None)
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
            ActionResult::ExileTarget(target) => {
                let ActiveTarget::Battlefield { id: target } = target else {
                    unreachable!()
                };
                Battlefield::exile(db, *target)
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
                counter,
            } => {
                match counter {
                    GainCounter::Single(counter) => {
                        CounterId::add_counters(db, *target, *counter, 1);
                    }
                    GainCounter::Dynamic(dynamic) => match dynamic {
                        DynamicCounter::X(counter) => {
                            let x = source.get_x(db);
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
            } => {
                card.move_to_stack(db, targets.clone(), Some(*from));
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
                                effect: Effect::Cascade,
                                threshold: None,
                                oracle_text: Default::default(),
                            }],
                            oracle_text: "Cascade".to_string(),
                        },
                        *card,
                        true,
                    );

                    id.move_to_stack(db, *card, Default::default());
                }
                PendingResults::default()
            }
            ActionResult::UpdateStackEntries(entries) => {
                for entry in entries.iter() {
                    match entry.ty {
                        Entry::Card(card) => {
                            card.move_to_stack(db, entry.targets.clone(), None);
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
                            Self::push_effect_results(
                                db,
                                *card,
                                card.controller(db),
                                (**effect).clone(),
                                &mut results,
                            );
                        }
                    }
                }

                results
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
                PendingResults::default()
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
            } => {
                card.mana_from_source(db, sources);
                let spent = all_players[card.controller(db)].spend_mana(mana, sources);
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
            ActionResult::Cascade { cascading, player } => {
                let mut results = PendingResults::default();
                results.cast_from(CastFrom::Exile);

                while let Some(card) = all_players[*player]
                    .deck
                    .exile_top_card(db, Some(ExileReason::Cascade))
                {
                    if !card.is_land(db) && card.cost(db).cmc() < *cascading {
                        results.push_choose_cast(card, false);
                        break;
                    }
                }

                results.push_settled(ActionResult::CascadeExileToBottomOfLibrary(*player));

                results
            }
            ActionResult::Discover { count, player } => {
                let mut results = PendingResults::default();
                results.cast_from(CastFrom::Exile);
                results.discovering();

                while let Some(card) = all_players[*player]
                    .deck
                    .exile_top_card(db, Some(ExileReason::Cascade))
                {
                    if !card.is_land(db) && card.cost(db).cmc() < *count {
                        results.push_choose_cast(card, false);
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

                let mut results = PendingResults::new(Source::Card(*source));
                results.push_choose_scry(cards);

                results
            }
            ActionResult::GainLife { target, count } => {
                all_players[*target].life_total += *count as i32;
                PendingResults::default()
            }
            ActionResult::Craft {
                transforming,
                targets,
            } => {
                transforming.move_to_exile(db, None);
                for target in targets {
                    let card = target.id().unwrap();
                    card.move_to_exile(db, None);
                }
                transforming.transform(db);
                let mut results = PendingResults::new(Source::Card(transforming.faceup_face(db)));
                move_card_to_battlefield(
                    db,
                    transforming.faceup_face(db),
                    transforming.faceup_face(db).etb_tapped(db),
                    &mut results,
                    None,
                );
                complete_add_from_exile(db, transforming.faceup_face(db), &mut results);
                transforming.move_to_limbo(db);
                results
            }
            ActionResult::DeclareAttackers { attackers, targets } => {
                let mut results = PendingResults::default();
                for (attacker, target) in attackers.iter().zip(targets.iter()) {
                    attacker.set_attacking(db, *target);
                    if !attacker.vigilance(db) {
                        results.extend(attacker.tap(db));
                    }
                }
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
                    results.extend(Battlefield::permanent_to_graveyard(db, card));
                }

                results
            }
            ActionResult::DestroyTarget(target) => {
                let ActiveTarget::Battlefield { id } = target else {
                    unreachable!()
                };

                Battlefield::permanent_to_graveyard(db, *id)
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

    pub fn permanent_to_graveyard(db: &mut Database, target: CardId) -> PendingResults {
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

    pub fn exile(db: &mut Database, target: CardId) -> PendingResults {
        target.move_to_exile(db, None);

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
                    token: Box::new(token),
                });
            }
            Effect::GainCounter(counter) => {
                results.push_settled(ActionResult::AddCounters {
                    source,
                    target: source,
                    counter,
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
            Effect::Cascade => {
                results.push_settled(ActionResult::Cascade {
                    cascading: source.cost(db).cmc(),
                    player: controller,
                });
            }
            Effect::Discover(count) => results.push_settled(ActionResult::Discover {
                count,
                player: source.controller(db),
            }),
            Effect::Scry(count) => {
                results.push_settled(ActionResult::Scry(source, count));
            }
            Effect::ForEachManaOfSource(ForEachManaOfSource {
                source: mana_source,
                effect,
            }) => {
                results.push_settled(ActionResult::ForEachManaOfSource {
                    card: source,
                    source: mana_source,
                    effect,
                });
            }
            Effect::GainLife(count) => {
                results.push_settled(ActionResult::GainLife {
                    target: controller,
                    count,
                });
            }
            Effect::DestroyEach(DestroyEach { restrictions }) => {
                results.push_settled(ActionResult::DestroyEach(source, restrictions));
            }
            Effect::CopyOfAnyCreatureNonTargeting
            | Effect::TutorLibrary(_)
            | Effect::CounterSpell { .. }
            | Effect::DealDamage(_)
            | Effect::Equip(_)
            | Effect::ExileTargetCreature
            | Effect::ExileTargetCreatureManifestTopOfLibrary
            | Effect::Mill(_)
            | Effect::ModifyTarget(_)
            | Effect::ReturnFromGraveyardToBattlefield(_)
            | Effect::ReturnFromGraveyardToLibrary(_)
            | Effect::CreateTokenCopy { .. }
            | Effect::TargetToTopOfLibrary { .. }
            | Effect::UntapTarget
            | Effect::Craft(_)
            | Effect::TargetGainsCounters(_)
            | Effect::DestroyTarget(_) => {
                let valid_targets = source.targets_for_effect(
                    db,
                    controller,
                    &effect,
                    results.all_currently_targeted(),
                );
                results.push_choose_targets(ChooseTargets::new(
                    TargetSource::Effect(effect),
                    valid_targets,
                ));
            }
        }
    }

    fn push_effect_results_with_target_from_top_of_library(
        db: &Database,
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
                    token: Box::new(token.clone()),
                });
            }
            Effect::GainCounter(counter) => {
                results.push_settled(ActionResult::AddCounters {
                    source,
                    target: source,
                    counter: *counter,
                });
            }
            Effect::CreateTokenCopy { modifiers } => {
                results.push_settled(ActionResult::CreateTokenCopyOf {
                    target,
                    controller: source.controller(db),
                    modifiers: modifiers.clone(),
                })
            }
            Effect::Cascade => results.push_settled(ActionResult::Cascade {
                cascading: source.cost(db).cmc(),
                player: source.controller(db),
            }),
            &Effect::TutorLibrary(_)
            | Effect::CounterSpell { .. }
            | Effect::Scry(_)
            | Effect::DealDamage(_)
            | Effect::Equip(_)
            | Effect::ExileTargetCreature
            | Effect::ExileTargetCreatureManifestTopOfLibrary
            | Effect::Mill(_)
            | Effect::ModifyTarget(_)
            | Effect::ReturnFromGraveyardToBattlefield(_)
            | Effect::ReturnFromGraveyardToLibrary(_)
            | Effect::BattlefieldModifier(_)
            | Effect::CopyOfAnyCreatureNonTargeting
            | Effect::RevealEachTopOfLibrary(_)
            | Effect::UntapThis
            | Effect::ReturnSelfToHand
            | Effect::TargetToTopOfLibrary { .. }
            | Effect::UntapTarget
            | Effect::TargetGainsCounters(_)
            | Effect::ForEachManaOfSource(_)
            | Effect::Discover(_)
            | Effect::GainLife(_)
            | Effect::Craft(_)
            | Effect::DestroyEach(_)
            | Effect::DestroyTarget(_) => {
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
            if !card.passes_restrictions(db, source_card, controller, &source_card.restrictions(db))
            {
                continue;
            }

            if !card.types_intersect(db, types) {
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
            StaticAbility::GreenCannotBeCountered { .. } => {}
            StaticAbility::BattlefieldModifier(modifier) => {
                let modifier = ModifierId::upload_temporary_modifier(db, source_card_id, &modifier);
                results.push_settled(ActionResult::AddModifier { modifier })
            }
            StaticAbility::ExtraLandsPerTurn(_) => {}
            StaticAbility::ForceEtbTapped(_) => {}
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
