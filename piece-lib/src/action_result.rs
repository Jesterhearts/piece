use std::vec::IntoIter;

use itertools::Itertools;
use rand::{seq::SliceRandom, thread_rng};
use tracing::Level;

use crate::{
    abilities::Ability,
    battlefield::{
        complete_add_from_exile, complete_add_from_graveyard, complete_add_from_library,
        complete_add_from_stack_or_hand, move_card_to_battlefield, Battlefields,
    },
    effects::EffectBehaviors,
    in_play::{CardId, CastFrom, Database, ExileReason, ModifierId},
    library::Library,
    log::{Log, LogEntry, LogId},
    pending_results::{
        examine_top_cards::{self, ExamineCards},
        PendingResults,
    },
    player::{mana_pool::SpendReason, Controller, Owner, Player},
    protogen::{
        abilities::TriggeredAbility,
        counters::Counter,
        effects::{
            create_token::Token,
            effect,
            examine_top_cards::Dest,
            replacement_effect::Replacing,
            target_gains_counters::{self, dynamic::Dynamic},
            BattleCry, BattlefieldModifier, Cascade, Duration, Effect, ModifyBattlefield,
            ReplacementEffect, RevealEachTopOfLibrary,
        },
        mana::{Mana, ManaRestriction, ManaSource},
        targets::{restriction, Location, Restriction},
        triggers::{self, Trigger, TriggerSource},
        types::Type,
    },
    stack::{ActiveTarget, Entry, Stack, StackEntry, StackId},
    types::TypeSet,
};

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
        count: target_gains_counters::Count,
        counter: protobuf::EnumOrUnknown<Counter>,
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
    CloneCard {
        cloning: CardId,
        cloned: CardId,
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
        modifiers: Vec<ModifyBattlefield>,
    },
    DamageTarget {
        quantity: u32,
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
        count: u32,
    },
    Discover {
        source: CardId,
        count: u32,
        player: Controller,
    },
    DrawCards {
        target: Controller,
        count: usize,
    },
    ExamineTopCards {
        destinations: Vec<Dest>,
        count: u32,
        controller: Controller,
    },
    ExileGraveyard {
        target: ActiveTarget,
        source: CardId,
    },
    ExileTarget {
        source: CardId,
        target: ActiveTarget,
        duration: protobuf::EnumOrUnknown<Duration>,
        reason: Option<ExileReason>,
    },
    Explore {
        target: ActiveTarget,
    },
    ForEachManaOfSource {
        card: CardId,
        source: protobuf::EnumOrUnknown<ManaSource>,
        effect: protobuf::MessageField<Effect>,
    },
    GainLife {
        target: Controller,
        count: u32,
    },
    GainMana {
        gain: Vec<protobuf::EnumOrUnknown<Mana>>,
        target: Controller,
        source: protobuf::EnumOrUnknown<ManaSource>,
        restriction: protobuf::EnumOrUnknown<ManaRestriction>,
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
        count: u32,
    },
    ManifestTopOfLibrary(Controller),
    Mill {
        count: u32,
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
        counter: protobuf::EnumOrUnknown<Counter>,
        count: usize,
    },
    ReturnFromBattlefieldToLibrary {
        target: ActiveTarget,
        under_cards: u32,
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
    Scry(CardId, u32),
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

impl ActionResult {
    #[instrument(skip(db), level = Level::DEBUG)]
    pub(crate) fn apply_action_results(
        db: &mut Database,
        results: &[ActionResult],
    ) -> PendingResults {
        let mut pending = PendingResults::default();

        for result in results.iter() {
            pending.extend(result.apply(db));
        }

        let entries = Log::current_session(db).to_vec();
        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::ONE_OR_MORE_TAPPED) {
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
    fn apply(&self, db: &mut Database) -> PendingResults {
        match self {
            ActionResult::Discard(card) => {
                assert!(card.is_in_location(db, Location::IN_HAND));
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
                Battlefields::permanent_to_graveyard(db, *card_id)
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
                        db.active_triggers_of_source(TriggerSource::ABILITY_ACTIVATED)
                    {
                        results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                    }
                } else {
                    Log::triggered(db, *source);
                }
                results.extend(Stack::push_ability(
                    db,
                    *source,
                    ability.clone(),
                    targets.clone(),
                ));

                results
            }
            ActionResult::CloneCard { cloning, cloned } => {
                cloning.clone_card(db, *cloned);
                PendingResults::default()
            }
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
                            pending.extend(Battlefields::library_to_graveyard(db, card_id));
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
                    pending.extend(Battlefields::add_from_stack_or_hand(db, *target, None));
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

                Library::place_under_top(db, db[*target].owner, *target, *under_cards as usize);
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
                        source.enum_value().unwrap(),
                        restriction.enum_value().unwrap(),
                    )
                }
                PendingResults::default()
            }
            ActionResult::CreateToken { source, token } => {
                let mut results = PendingResults::default();

                let mut replacements = db
                    .replacement_abilities_watching(Replacing::TOKEN_CREATION)
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
                let Some(target) = target.id(db) else {
                    unreachable!()
                };
                if let Duration::UNTIL_SOURCE_LEAVES_BATTLEFIELD = duration.enum_value().unwrap() {
                    if !source.is_in_location(db, Location::ON_BATTLEFIELD) {
                        return PendingResults::default();
                    }
                }

                Battlefields::exile(db, *source, target, *reason, duration.enum_value().unwrap())
            }
            ActionResult::DamageTarget { quantity, target } => {
                match target {
                    ActiveTarget::Battlefield { id } => {
                        id.mark_damage(db, *quantity);
                    }
                    ActiveTarget::Player { id } => {
                        db.all_players[*id].life_total -= *quantity as i32;
                    }
                    _ => unreachable!(),
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
                *db[*target]
                    .counters
                    .entry(counter.enum_value().unwrap())
                    .or_default() = db[*target]
                    .counters
                    .entry(counter.enum_value().unwrap())
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
                    target_gains_counters::Count::Single(_) => {
                        *db[*target]
                            .counters
                            .entry(counter.enum_value().unwrap())
                            .or_default() += 1;
                    }
                    target_gains_counters::Count::Multiple(count) => {
                        *db[*target]
                            .counters
                            .entry(counter.enum_value().unwrap())
                            .or_default() += count.count as usize;
                    }
                    target_gains_counters::Count::Dynamic(dynamic) => {
                        match dynamic.dynamic.as_ref().unwrap() {
                            Dynamic::X(_) => {
                                let x = source.get_x(db);
                                if x > 0 {
                                    *db[*target]
                                        .counters
                                        .entry(counter.enum_value().unwrap())
                                        .or_default() += x;
                                }
                            }
                            Dynamic::LeftBattlefieldThisTurn(left) => {
                                let cards = CardId::left_battlefield_this_turn(db);
                                let x = cards
                                    .filter(|card| {
                                        card.passes_restrictions(
                                            db,
                                            LogId::current(db),
                                            *source,
                                            &left.restrictions,
                                        )
                                    })
                                    .count();
                                if x > 0 {
                                    *db[*target]
                                        .counters
                                        .entry(counter.enum_value().unwrap())
                                        .or_default() += x;
                                }
                            }
                        }
                    }
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

                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::CAST) {
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
                            trigger: protobuf::MessageField::some(Trigger {
                                source: TriggerSource::CAST.into(),
                                from: triggers::Location::HAND.into(),
                                restrictions: vec![Restriction {
                                    restriction: Some(restriction::Restriction::from(
                                        restriction::Controller {
                                            controller: Some(
                                                restriction::controller::Controller::Self_(
                                                    Default::default(),
                                                ),
                                            ),
                                            ..Default::default()
                                        },
                                    )),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            }),
                            effects: vec![Effect {
                                effect: Some(effect::Effect::from(Cascade::default())),
                                ..Default::default()
                            }],
                            oracle_text: "Cascade".to_string(),
                            ..Default::default()
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
            ActionResult::HandFromBattlefield(card) => Battlefields::permanent_to_hand(db, *card),
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
                    for effect in reveal.for_each.if_none.effects.iter() {
                        effect.effect.as_ref().unwrap().push_pending_behavior(
                            db,
                            *source,
                            controller,
                            &mut results,
                        );
                    }
                } else {
                    for target in revealed {
                        for effect in reveal.for_each.effects.iter() {
                            effect
                                .effect
                                .as_ref()
                                .unwrap()
                                .push_behavior_from_top_of_library(
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
                if let Some(from_source) = db[*card].sourced_mana.get(&source.enum_value().unwrap())
                {
                    for _ in 0..*from_source {
                        effect.effect.as_ref().unwrap().push_pending_behavior(
                            db,
                            *card,
                            db[*card].controller,
                            &mut results,
                        );
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
                    .replacement_abilities_watching(Replacing::TOKEN_CREATION)
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
            ActionResult::MoveFromLibraryToGraveyard(card) => {
                Battlefields::library_to_graveyard(db, *card)
            }
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
                let stun = db[*target].counters.entry(Counter::STUN).or_default();
                if *stun > 0 {
                    *stun -= 1;
                } else {
                    target.untap(db);
                }

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
                    if !card.is_land(db) && card.faceup_face(db).cost.cmc() < *count as usize {
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
                db.all_players[*target].life_gained_this_turn += *count;
                PendingResults::default()
            }
            ActionResult::DeclareAttackers { attackers, targets } => {
                let mut results = PendingResults::default();
                for (attacker, target) in attackers.iter().zip(targets.iter()) {
                    db[*attacker].attacking = Some(*target);

                    let listeners = db.active_triggers_of_source(TriggerSource::ATTACKS);
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
                                trigger: protobuf::MessageField::some(Trigger {
                                    source: TriggerSource::ATTACKS.into(),
                                    from: triggers::Location::ANYWHERE.into(),
                                    restrictions: vec![Restriction {
                                        restriction: Some(restriction::Restriction::from(
                                            restriction::Controller {
                                                controller: Some(
                                                    restriction::controller::Controller::Self_(
                                                        Default::default(),
                                                    ),
                                                ),
                                                ..Default::default()
                                            },
                                        )),
                                        ..Default::default()
                                    }],
                                    ..Default::default()
                                }),
                                effects: vec![Effect {
                                    effect: Some(effect::Effect::from(BattleCry::default())),
                                    ..Default::default()
                                }],
                                oracle_text: "Battle cry".to_string(),
                                ..Default::default()
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
                let explorer = target.id(db).unwrap();
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
                    card.move_to_exile(db, *source, None, Duration::PERMANENTLY)
                }

                PendingResults::default()
            }
            ActionResult::ReturnTransformed {
                target,
                enters_tapped,
            } => {
                target.transform(db);
                let mut results = PendingResults::default();
                let location = if target.is_in_location(db, Location::IN_EXILE) {
                    Location::IN_EXILE
                } else if target.is_in_location(db, Location::IN_GRAVEYARD) {
                    Location::IN_GRAVEYARD
                } else {
                    unreachable!("unexpected location {:?}", target.target_from_location(db))
                };
                move_card_to_battlefield(db, *target, *enters_tapped, &mut results, None);
                match location {
                    Location::IN_EXILE => complete_add_from_exile(db, *target, &mut results),
                    Location::IN_GRAVEYARD => {
                        complete_add_from_graveyard(db, *target, &mut results)
                    }
                    _ => unreachable!(),
                }

                results
            }
            ActionResult::Transform { target } => {
                target.transform(db);

                PendingResults::default()
            }
            ActionResult::BanAttacking(player) => {
                db.all_players[*player].ban_attacking_this_turn = true;
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
                        effect.effect.as_ref().unwrap().push_pending_behavior(
                            db,
                            *source,
                            *controller,
                            &mut results,
                        );
                    }
                }

                results
            }
        }
    }
}

#[instrument(skip(db, modifiers, results))]
pub(crate) fn create_token_copy_with_replacements(
    db: &mut Database,
    source: CardId,
    copying: CardId,
    modifiers: &[ModifyBattlefield],
    replacements: &mut IntoIter<(CardId, ReplacementEffect)>,
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
                effect.effect.as_ref().unwrap().replace_token_creation(
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
                    modifier: protobuf::MessageField::some(modifier.clone()),
                    duration: Duration::UNTIL_SOURCE_LEAVES_BATTLEFIELD.into(),
                    ..Default::default()
                },
            );
            modifier.activate(&mut db.modifiers);

            token.apply_modifier(db, modifier);
        }

        token.apply_modifiers_layered(db);
        results.extend(Battlefields::add_from_stack_or_hand(db, token, None));
    }
}
