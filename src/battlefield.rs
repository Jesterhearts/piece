use std::collections::HashSet;

use bevy_ecs::{entity::Entity, query::With};
use itertools::Itertools;

use crate::{
    abilities::{compute_mana_gain, Ability, ETBAbility, StaticAbility},
    card::Color,
    controller::ControllerRestriction,
    cost::AdditionalCost,
    effects::{
        effect_duration::UntilEndOfTurn, Destination, Mill, ReturnFromGraveyardToBattlefield,
        ReturnFromGraveyardToLibrary, TutorLibrary,
    },
    in_play::{
        all_cards, cards, AbilityId, Active, CardId, Database, InGraveyard, InLibrary, ModifierId,
        OnBattlefield, TriggerId,
    },
    player::{AllPlayers, Controller, Owner},
    stack::{ActiveTarget, Stack},
    targets::Restriction,
    triggers::{self, trigger_source},
    types::Type,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedActionResult {
    TapPermanent(CardId),
    PermanentToGraveyard(CardId),
    LoseLife {
        target: Controller,
        count: usize,
    },
    AddAbilityToStack {
        source: CardId,
        ability: AbilityId,
        valid_targets: HashSet<ActiveTarget>,
    },
    GainMana {
        source: CardId,
        ability: AbilityId,
        mode: Option<usize>,
    },
    AddTriggerToStack(TriggerId, CardId),
    CloneCreatureNonTargeting {
        source: CardId,
        valid_targets: Vec<CardId>,
    },
    AddModifier {
        modifier: ModifierId,
    },
    Mill {
        count: usize,
        valid_targets: HashSet<Owner>,
    },
    ReturnFromGraveyardToLibrary {
        source: CardId,
        count: usize,
        controller: ControllerRestriction,
        types: HashSet<Type>,
        valid_targets: Vec<CardId>,
    },
    ReturnFromGraveyardToBattlefield {
        source: CardId,
        count: usize,
        types: HashSet<Type>,
        valid_targets: Vec<CardId>,
    },
    TutorLibrary {
        source: CardId,
        destination: Destination,
        targets: Vec<CardId>,
        reveal: bool,
        restrictions: Vec<Restriction>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    TapPermanent(CardId),
    PermanentToGraveyard(CardId),
    AddAbilityToStack {
        source: CardId,
        ability: AbilityId,
        targets: HashSet<ActiveTarget>,
    },
    AddTriggerToStack(TriggerId, CardId),
    CloneCreatureNonTargeting {
        source: CardId,
        target: Option<CardId>,
    },
    AddModifier {
        modifier: ModifierId,
    },
    Mill {
        count: usize,
        targets: HashSet<Owner>,
    },
    ReturnFromGraveyardToLibrary {
        targets: Vec<CardId>,
    },
    ReturnFromGraveyardToBattlefield {
        targets: Vec<CardId>,
    },
    LoseLife {
        target: Controller,
        count: usize,
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

        for card in on_battlefield {
            let types = card.types(db);
            if types.contains(&Type::Creature) {
                results.push(card);
            }
        }

        results
    }

    #[must_use]
    pub fn add_from_stack(
        db: &mut Database,
        source_card_id: CardId,
        targets: Vec<CardId>,
    ) -> Vec<UnresolvedActionResult> {
        let mut results = vec![];

        if let Some(aura) = source_card_id.aura(db) {
            for target in targets.iter() {
                target.apply_aura(db, aura);
            }
        }

        for etb in source_card_id.etb_abilities(db).iter() {
            match etb {
                ETBAbility::CopyOfAnyCreature => {
                    assert!(targets.is_empty());

                    results.push(UnresolvedActionResult::CloneCreatureNonTargeting {
                        source: source_card_id,
                        valid_targets: Self::creatures(db),
                    });
                }
                ETBAbility::Mill(Mill { count, target }) => {
                    let targets = match target {
                        ControllerRestriction::Any => AllPlayers::all_players(db),
                        ControllerRestriction::You => {
                            HashSet::from([source_card_id.controller(db).into()])
                        }
                        ControllerRestriction::Opponent => {
                            let mut all = AllPlayers::all_players(db);
                            all.remove(&source_card_id.controller(db).into());
                            all
                        }
                    };

                    results.push(UnresolvedActionResult::Mill {
                        count: *count,
                        valid_targets: targets,
                    })
                }
                ETBAbility::ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary {
                    count,
                    controller,
                    types,
                }) => {
                    let valid_targets =
                        compute_graveyard_targets(db, *controller, source_card_id, types);

                    results.push(UnresolvedActionResult::ReturnFromGraveyardToLibrary {
                        source: source_card_id,
                        count: *count,
                        controller: *controller,
                        types: types.clone(),
                        valid_targets,
                    });
                }
                ETBAbility::ReturnFromGraveyardToBattlefield(
                    ReturnFromGraveyardToBattlefield { count, types },
                ) => {
                    let valid_targets = compute_graveyard_targets(
                        db,
                        ControllerRestriction::You,
                        source_card_id,
                        types,
                    );

                    results.push(UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
                        source: source_card_id,
                        count: *count,
                        types: types.clone(),
                        valid_targets,
                    });
                }
                ETBAbility::TutorLibrary(TutorLibrary {
                    restrictions,
                    destination,
                    reveal,
                }) => {
                    let controller = source_card_id.controller(db);
                    let targets = compute_deck_targets(db, controller, restrictions);
                    results.push(UnresolvedActionResult::TutorLibrary {
                        source: source_card_id,
                        destination: *destination,
                        targets,
                        reveal: *reveal,
                        restrictions: restrictions.clone(),
                    });
                }
            }
        }

        for ability in source_card_id.static_abilities(db) {
            match ability {
                StaticAbility::GreenCannotBeCountered { .. } => {}
                StaticAbility::BattlefieldModifier(modifier) => {
                    let modifier =
                        ModifierId::upload_temporary_modifier(db, source_card_id, &modifier);
                    results.push(UnresolvedActionResult::AddModifier { modifier })
                }
                StaticAbility::ExtraLandsPerTurn(_) => {}
            }
        }

        source_card_id.move_to_battlefield(db);

        for trigger in
            TriggerId::active_triggers_of_source::<trigger_source::EntersTheBattlefield>(db)
        {
            if matches!(trigger.location_from(db), triggers::Location::Anywhere) {
                let for_types = trigger.for_types(db);
                if source_card_id.types_intersect(db, &for_types) {
                    for source in trigger.listeners(db) {
                        results.push(UnresolvedActionResult::AddTriggerToStack(trigger, source))
                    }
                }
            }
        }

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        results
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

    pub fn select_card(db: &mut Database, index: usize) -> CardId {
        cards::<OnBattlefield>(db)[index]
    }

    #[must_use]
    pub fn activate_ability(
        db: &mut Database,
        all_players: &mut AllPlayers,
        card: CardId,
        index: usize,
    ) -> Vec<UnresolvedActionResult> {
        if Stack::split_second(db) {
            return vec![];
        }

        let mut results = vec![];

        let ability_id = card.activated_abilities(db)[index];
        let ability = ability_id.ability(db);

        if ability.cost().tap {
            if card.tapped(db) {
                return vec![];
            }

            results.push(UnresolvedActionResult::TapPermanent(card));
        }

        for cost in ability.cost().additional_cost.iter() {
            match cost {
                AdditionalCost::SacrificeThis => {
                    if !card.can_be_sacrificed(db) {
                        return vec![];
                    }

                    results.push(UnresolvedActionResult::PermanentToGraveyard(card));
                }
            }
        }

        if !all_players[card.controller(db)].spend_mana(&ability.cost().mana_cost) {
            return vec![];
        }

        if let Ability::Mana(_) = ability {
            results.push(UnresolvedActionResult::GainMana {
                source: card,
                ability: ability_id,
                mode: None,
            });
        } else {
            results.push(UnresolvedActionResult::AddAbilityToStack {
                source: card,
                ability: ability_id,
                valid_targets: card.valid_targets(db),
            });
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

    /// Attempts to automatically resolve any unresolved actions and _recomputes targets for pending actions.
    #[must_use]
    pub fn maybe_resolve(
        db: &mut Database,
        all_players: &mut AllPlayers,
        results: Vec<UnresolvedActionResult>,
    ) -> Vec<UnresolvedActionResult> {
        let mut pending = vec![];
        let mut resolved = vec![];

        Self::attempt_to_resolve(db, all_players, results, &mut resolved, &mut pending);
        let applied_results = Self::apply_action_results(db, all_players, resolved);
        if applied_results.is_empty() && !pending.is_empty() {
            let mut new_pending = vec![];
            let mut resolved = vec![];
            Self::attempt_to_resolve(db, all_players, pending, &mut resolved, &mut new_pending);
            new_pending.extend(Self::apply_action_results(db, all_players, resolved));

            pending = new_pending;
        } else {
            pending.extend(applied_results);
        }

        pending
    }

    #[must_use]
    pub fn apply_action_results(
        db: &mut Database,
        all_players: &mut AllPlayers,
        results: Vec<ActionResult>,
    ) -> Vec<UnresolvedActionResult> {
        let mut pending = vec![];

        for result in results {
            pending.extend(Self::apply_action_result(db, all_players, result));
        }

        for card in all_cards(db) {
            card.apply_modifiers_layered(db);
        }

        pending
    }

    #[must_use]
    fn apply_action_result(
        db: &mut Database,
        all_players: &mut AllPlayers,
        result: ActionResult,
    ) -> Vec<UnresolvedActionResult> {
        match result {
            ActionResult::TapPermanent(card_id) => {
                card_id.tap(db);
            }
            ActionResult::PermanentToGraveyard(card_id) => {
                return Self::permanent_to_graveyard(db, card_id);
            }
            ActionResult::AddAbilityToStack {
                source,
                ability,
                targets,
            } => {
                ability.move_to_stack(db, source, targets);
            }
            ActionResult::AddTriggerToStack(trigger, source) => {
                trigger.move_to_stack(db, source, Default::default());
            }
            ActionResult::CloneCreatureNonTargeting { source, target } => {
                if let Some(target) = target {
                    source.clone_card(db, target);
                }
            }
            ActionResult::AddModifier { modifier } => {
                modifier.activate(db);
            }
            ActionResult::Mill { count, targets } => {
                let mut pending = vec![];
                for target in targets {
                    let deck = &mut all_players[target].deck;
                    for _ in 0..count {
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
                    all_players[target.owner(db)].deck.place_on_top(db, target);
                }
            }
            ActionResult::ReturnFromGraveyardToBattlefield { targets } => {
                let mut pending = vec![];
                for target in targets {
                    pending.extend(Self::add_from_stack(db, target, vec![]));
                }

                return pending;
            }
            ActionResult::LoseLife { target, count } => {
                all_players[target].life_total -= count as i32;
            }
        }

        vec![]
    }

    #[must_use]
    pub fn permanent_to_graveyard(
        db: &mut Database,
        target: CardId,
    ) -> Vec<UnresolvedActionResult> {
        let mut pending = vec![];

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(
                trigger.location_from(db),
                triggers::Location::Anywhere | triggers::Location::Battlefield
            ) {
                let for_types = trigger.for_types(db);
                if target.types_intersect(db, &for_types) {
                    for source in trigger.listeners(db) {
                        pending.push(UnresolvedActionResult::AddTriggerToStack(trigger, source))
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

    #[must_use]
    pub fn library_to_graveyard(db: &mut Database, target: CardId) -> Vec<UnresolvedActionResult> {
        let mut pending = vec![];

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(
                trigger.location_from(db),
                triggers::Location::Anywhere | triggers::Location::Library
            ) {
                let for_types = trigger.for_types(db);
                if target.types_intersect(db, &for_types) {
                    for source in trigger.listeners(db) {
                        pending.push(UnresolvedActionResult::AddTriggerToStack(trigger, source))
                    }
                }
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    #[must_use]
    pub fn stack_to_graveyard(db: &mut Database, target: CardId) -> Vec<UnresolvedActionResult> {
        let mut pending = vec![];

        for trigger in TriggerId::active_triggers_of_source::<trigger_source::PutIntoGraveyard>(db)
        {
            if matches!(trigger.location_from(db), triggers::Location::Anywhere) {
                let for_types = trigger.for_types(db);
                if target.types_intersect(db, &for_types) {
                    for source in trigger.listeners(db) {
                        pending.push(UnresolvedActionResult::AddTriggerToStack(trigger, source))
                    }
                }
            }
        }

        target.move_to_graveyard(db);

        pending
    }

    #[must_use]
    pub fn exile(db: &mut Database, target: CardId) -> Vec<UnresolvedActionResult> {
        target.move_to_exile(db);

        vec![]
    }

    fn attempt_to_resolve(
        db: &mut Database,
        all_players: &mut AllPlayers,
        results: Vec<UnresolvedActionResult>,
        resolved: &mut Vec<ActionResult>,
        pending: &mut Vec<UnresolvedActionResult>,
    ) {
        for result in results {
            match result {
                UnresolvedActionResult::TapPermanent(cardid) => {
                    resolved.push(ActionResult::TapPermanent(cardid));
                }
                UnresolvedActionResult::PermanentToGraveyard(cardid) => {
                    resolved.push(ActionResult::PermanentToGraveyard(cardid));
                }
                UnresolvedActionResult::AddAbilityToStack {
                    source,
                    ability,
                    valid_targets,
                } => {
                    let controller = source.controller(db);
                    let wants_targets: usize = ability
                        .effects(db)
                        .iter()
                        .map(|effect| effect.wants_targets(db, controller))
                        .max()
                        .unwrap_or_default();

                    if wants_targets >= valid_targets.len() {
                        resolved.push(ActionResult::AddAbilityToStack {
                            source,
                            ability,
                            targets: valid_targets,
                        });
                    } else {
                        let mut valid_targets = Default::default();
                        let creatures = Self::creatures(db);
                        source.targets_for_ability(db, ability, &creatures, &mut valid_targets);

                        pending.push(UnresolvedActionResult::AddAbilityToStack {
                            source,
                            ability,
                            valid_targets,
                        });
                    }
                }
                UnresolvedActionResult::AddTriggerToStack(trigger, source) => {
                    resolved.push(ActionResult::AddTriggerToStack(trigger, source));
                }
                UnresolvedActionResult::CloneCreatureNonTargeting {
                    source,
                    valid_targets,
                } => {
                    pending.push(UnresolvedActionResult::CloneCreatureNonTargeting {
                        source,
                        valid_targets,
                    });
                }
                UnresolvedActionResult::AddModifier { modifier } => {
                    resolved.push(ActionResult::AddModifier { modifier });
                }
                UnresolvedActionResult::Mill {
                    count,
                    valid_targets,
                } => {
                    if valid_targets.len() == 1 {
                        resolved.push(ActionResult::Mill {
                            count,
                            targets: valid_targets,
                        });
                    } else {
                        pending.push(UnresolvedActionResult::Mill {
                            count,
                            valid_targets,
                        });
                    }
                }
                UnresolvedActionResult::ReturnFromGraveyardToLibrary {
                    source,
                    count,
                    controller,
                    types,
                    valid_targets,
                } => {
                    if valid_targets.len() == count {
                        resolved.push(ActionResult::ReturnFromGraveyardToLibrary {
                            targets: valid_targets,
                        });
                    } else {
                        let valid_targets =
                            compute_graveyard_targets(db, controller, source, &types);
                        pending.push(UnresolvedActionResult::ReturnFromGraveyardToLibrary {
                            source,
                            count,
                            controller,
                            types,
                            valid_targets,
                        })
                    }
                }
                UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
                    source,
                    count,
                    types,
                    valid_targets,
                } => {
                    if valid_targets.len() == count {
                        resolved.push(ActionResult::ReturnFromGraveyardToBattlefield {
                            targets: valid_targets,
                        });
                    } else {
                        let valid_targets = compute_graveyard_targets(
                            db,
                            ControllerRestriction::You,
                            source,
                            &types,
                        );
                        pending.push(UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
                            source,
                            count,
                            types,
                            valid_targets,
                        })
                    }
                }
                UnresolvedActionResult::TutorLibrary {
                    source,
                    destination,
                    targets: _,
                    reveal,
                    restrictions,
                } => {
                    let controller = source.controller(db);
                    let targets = compute_deck_targets(db, controller, &restrictions);

                    pending.push(UnresolvedActionResult::TutorLibrary {
                        source,
                        destination,
                        targets,
                        reveal,
                        restrictions,
                    });
                }
                UnresolvedActionResult::GainMana {
                    source,
                    ability,
                    mode,
                } => {
                    if let Some(mana) = compute_mana_gain(&ability.gain_mana_ability(db).gain, mode)
                    {
                        for (mana, count) in mana {
                            for _ in 0..count {
                                all_players[source.controller(db)].mana_pool.apply(mana);
                            }
                        }
                    } else {
                        pending.push(UnresolvedActionResult::GainMana {
                            source,
                            ability,
                            mode,
                        });
                    }
                }
                UnresolvedActionResult::LoseLife { target, count } => {
                    resolved.push(ActionResult::LoseLife { target, count })
                }
            }
        }
    }
}

fn compute_deck_targets(
    db: &mut Database,
    player: Controller,
    restrictions: &[Restriction],
) -> Vec<CardId> {
    let mut results = vec![];

    for card in player.get_cards::<InLibrary>(db) {
        if !card.passes_restrictions(db, card, player, ControllerRestriction::You, restrictions) {
            continue;
        }

        results.push(card);
    }

    results
}

fn compute_graveyard_targets(
    db: &mut Database,
    controller: ControllerRestriction,
    source_card: CardId,
    types: &HashSet<Type>,
) -> Vec<CardId> {
    let targets = match controller {
        ControllerRestriction::Any => AllPlayers::all_players(db),
        ControllerRestriction::You => HashSet::from([source_card.controller(db).into()]),
        ControllerRestriction::Opponent => {
            let mut all = AllPlayers::all_players(db);
            all.remove(&source_card.controller(db).into());
            all
        }
    };
    let mut target_cards = vec![];

    for target in targets {
        let cards_in_graveyard = target.get_cards::<InGraveyard>(db);
        for card in cards_in_graveyard {
            if card.types_intersect(db, types) {
                target_cards.push(card);
            }
        }
    }

    target_cards
}
