use std::collections::HashSet;

use indoc::indoc;
use rusqlite::Connection;

use crate::{
    abilities::{ETBAbility, StaticAbility},
    card::Color,
    controller::Controller,
    cost::AdditionalCost,
    effects::{
        Destination, EffectDuration, Mill, ReturnFromGraveyardToBattlefield,
        ReturnFromGraveyardToLibrary, Token, TriggeredEffect, TutorLibrary,
    },
    in_play::{AbilityId, CardId, Location, ModifierId, TriggerId},
    player::{AllPlayers, PlayerId},
    stack::{ActiveTarget, Stack},
    targets::Restriction,
    triggers::{self, TriggerSource},
    types::Type,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedActionResult {
    TapPermanent(CardId),
    PermanentToGraveyard(CardId),
    AddToStack {
        ability: AbilityId,
        valid_targets: HashSet<ActiveTarget>,
    },
    CloneCreatureNonTargeting {
        source: CardId,
        valid_targets: Vec<CardId>,
    },
    AddModifier {
        modifier: ModifierId,
    },
    Mill {
        count: usize,
        valid_targets: HashSet<PlayerId>,
    },
    ReturnFromGraveyardToLibrary {
        source: CardId,
        count: usize,
        controller: Controller,
        types: HashSet<Type>,
        valid_targets: Vec<CardId>,
    },
    ReturnFromGraveyardToBattlefield {
        source: CardId,
        count: usize,
        types: HashSet<Type>,
        valid_targets: Vec<CardId>,
    },
    CreateToken {
        source: CardId,
        token: Token,
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
    AddToStack {
        ability: AbilityId,
        targets: HashSet<ActiveTarget>,
    },
    CloneCreatureNonTargeting {
        source: CardId,
        target: Option<CardId>,
    },
    AddModifier {
        modifier: ModifierId,
    },
    Mill {
        count: usize,
        targets: HashSet<PlayerId>,
    },
    ReturnFromGraveyardToLibrary {
        targets: Vec<CardId>,
    },
    ReturnFromGraveyardToBattlefield {
        targets: Vec<CardId>,
    },
    CreateToken {
        source: CardId,
        token: Token,
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
    pub fn is_empty(db: &Connection) -> anyhow::Result<bool> {
        let mut cards = db.prepare("SELECT NULL FROM cards WHERE location = (?1)")?;
        let mut rows = cards.query((serde_json::to_string(&Location::Battlefield)?,))?;
        Ok(rows.next()?.is_none())
    }

    pub fn no_modifiers(db: &Connection) -> anyhow::Result<bool> {
        let mut modifiers = db.prepare("SELECT NULL FROM modifiers WHERE active")?;
        let mut rows = modifiers.query(())?;
        Ok(rows.next()?.is_none())
    }

    pub fn creatures(db: &Connection) -> anyhow::Result<Vec<CardId>> {
        let mut on_battlefield = db.prepare(indoc! {"
                SELECT cardid
                FROM cards
                WHERE location = (?1)
        "})?;
        let mut results = vec![];

        let rows = on_battlefield
            .query_map((serde_json::to_string(&Location::Battlefield)?,), |row| {
                row.get(0)
            })?;
        for row in rows {
            let cardid: CardId = row?;
            let types = cardid.types(db)?;
            if types.contains(&Type::Creature) {
                results.push(cardid);
            }
        }

        Ok(results)
    }

    pub fn add(
        db: &Connection,
        source_card_id: CardId,
        targets: Vec<CardId>,
    ) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        let mut results = vec![];
        source_card_id.move_to_battlefield(db)?;

        if let Some(aura) = source_card_id.aura(db)? {
            for target in targets.iter() {
                target.apply_aura(db, aura)?;
            }
        }

        for etb in source_card_id.etb_abilities(db)? {
            match etb {
                ETBAbility::CopyOfAnyCreature => {
                    assert!(targets.is_empty());

                    results.push(UnresolvedActionResult::CloneCreatureNonTargeting {
                        source: source_card_id,
                        valid_targets: Self::creatures(db)?,
                    });
                }
                ETBAbility::Mill(Mill { count, target }) => {
                    let targets = match target {
                        Controller::Any => AllPlayers::all_players(db)?,
                        Controller::You => HashSet::from([source_card_id.controller(db)?]),
                        Controller::Opponent => {
                            // TODO this could probably be a query if I was smarter at sql
                            let mut all = AllPlayers::all_players(db)?;
                            all.remove(&source_card_id.controller(db)?);
                            all
                        }
                    };

                    results.push(UnresolvedActionResult::Mill {
                        count,
                        valid_targets: targets,
                    })
                }
                ETBAbility::ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary {
                    count,
                    controller,
                    types,
                }) => {
                    let valid_targets =
                        compute_graveyard_targets(db, controller, source_card_id, &types)?;

                    results.push(UnresolvedActionResult::ReturnFromGraveyardToLibrary {
                        source: source_card_id,
                        count,
                        controller,
                        types,
                        valid_targets,
                    });
                }
                ETBAbility::ReturnFromGraveyardToBattlefield(
                    ReturnFromGraveyardToBattlefield { count, types },
                ) => {
                    let valid_targets =
                        compute_graveyard_targets(db, Controller::You, source_card_id, &types)?;

                    results.push(UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
                        source: source_card_id,
                        count,
                        types,
                        valid_targets,
                    });
                }
                ETBAbility::TutorLibrary(TutorLibrary {
                    restrictions,
                    destination,
                    reveal,
                }) => {
                    let targets =
                        compute_deck_targets(db, source_card_id.controller(db)?, &restrictions)?;
                    results.push(UnresolvedActionResult::TutorLibrary {
                        source: source_card_id,
                        destination,
                        targets,
                        reveal,
                        restrictions,
                    });
                }
            }
        }

        for ability in source_card_id.static_abilities(db)? {
            match ability {
                StaticAbility::GreenCannotBeCountered { .. } => {}
                StaticAbility::Vigilance => {}
                StaticAbility::Flash => {}
                StaticAbility::BattlefieldModifier(modifier) => {
                    let modifier = ModifierId::upload_single_modifier(
                        db,
                        Some(source_card_id),
                        &modifier,
                        true,
                    )?;
                    results.push(UnresolvedActionResult::AddModifier { modifier })
                }
            }
        }

        Ok(results)
    }

    pub fn controlled_colors(db: &Connection, player: PlayerId) -> anyhow::Result<HashSet<Color>> {
        let mut cards =
            db.prepare("SELECT cardid FROM cards WHERE location = (?1) AND controller = (?2)")?;
        let rows = cards.query_map(
            (serde_json::to_string(&Location::Battlefield)?, player),
            |row| row.get::<_, CardId>(0),
        )?;

        let mut colors = HashSet::default();
        for row in rows {
            let card_colors = row?.colors(db)?;
            colors.extend(card_colors)
        }

        Ok(colors)
    }

    pub fn end_turn(db: &Connection) -> anyhow::Result<()> {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET modifying = NULL 
                WHERE modifiers.duration = (?1) AND modifiers.active
            "},
            (serde_json::to_string(&EffectDuration::UntilEndOfTurn)?,),
        )?;

        db.execute(
            indoc! {"
                UPDATE modifiers
                SET active = FALSE
                WHERE modifiers.duration = (?1) AND modifiers.active
            "},
            (serde_json::to_string(&EffectDuration::UntilEndOfTurn)?,),
        )?;

        Ok(())
    }

    pub fn check_sba(db: &Connection) -> anyhow::Result<Vec<ActionResult>> {
        let mut result = vec![];
        for card_id in Location::Battlefield.cards_in(db)? {
            let toughness = card_id.toughness(db)?;

            if toughness.is_some() && toughness <= Some(0) {
                result.push(ActionResult::PermanentToGraveyard(card_id));
            }

            let aura = card_id.aura(db)?;
            if aura.is_some() && !aura.unwrap().is_attached(db)? {
                result.push(ActionResult::PermanentToGraveyard(card_id));
            }
        }

        Ok(result)
    }

    pub fn select_card(db: &Connection, index: usize) -> anyhow::Result<CardId> {
        Ok(Location::Battlefield.cards_in(db)?[index])
    }

    pub fn activate_ability(
        db: &Connection,
        players: &mut AllPlayers,
        card: CardId,
        index: usize,
    ) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        if Stack::split_second(db)? {
            return Ok(vec![]);
        }

        let mut results = vec![];

        let ability_id = card.activated_abilities(db)?[index];
        let ability = ability_id.ability(db)?;

        if ability.cost.tap {
            if card.tapped(db)? {
                return Ok(vec![]);
            }

            results.push(UnresolvedActionResult::TapPermanent(card));
        }

        for cost in ability.cost.additional_cost.iter() {
            match cost {
                AdditionalCost::SacrificeThis => {
                    if !card.can_be_sacrificed(db)? {
                        return Ok(vec![]);
                    }

                    results.push(UnresolvedActionResult::PermanentToGraveyard(card));
                }
            }
        }

        if !players[card.controller(db)?].spend_mana(&ability.cost.mana_cost) {
            return Ok(vec![]);
        }

        results.push(UnresolvedActionResult::AddToStack {
            ability: ability_id,
            valid_targets: card.valid_targets(db)?,
        });

        Ok(results)
    }

    pub fn static_abilities(db: &Connection) -> anyhow::Result<Vec<(StaticAbility, PlayerId)>> {
        let mut result: Vec<(StaticAbility, PlayerId)> = Default::default();

        for card in Location::Battlefield.cards_in(db)? {
            let controller = card.controller(db)?;
            for ability in card.static_abilities(db)? {
                result.push((ability, controller));
            }
        }

        Ok(result)
    }

    /// Attempts to automatically resolve any unresolved actions and _recomputes targets for pending actions.
    pub fn maybe_resolve(
        db: &Connection,
        all_players: &mut AllPlayers,
        results: Vec<UnresolvedActionResult>,
    ) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        let mut pending = vec![];

        for result in results {
            match result {
                UnresolvedActionResult::TapPermanent(cardid) => {
                    pending.extend(Self::apply_action_result(
                        db,
                        all_players,
                        ActionResult::TapPermanent(cardid),
                    )?);
                }
                UnresolvedActionResult::PermanentToGraveyard(cardid) => {
                    pending.extend(Self::apply_action_result(
                        db,
                        all_players,
                        ActionResult::PermanentToGraveyard(cardid),
                    )?);
                }
                UnresolvedActionResult::AddToStack {
                    ability,
                    valid_targets,
                } => {
                    let wants_targets: usize = ability
                        .effects(db)?
                        .into_iter()
                        .map(|effect| effect.wants_targets())
                        .max()
                        .unwrap();
                    if wants_targets >= valid_targets.len() {
                        pending.extend(Self::apply_action_result(
                            db,
                            all_players,
                            ActionResult::AddToStack {
                                ability,
                                targets: valid_targets,
                            },
                        )?);
                    } else {
                        pending.push(UnresolvedActionResult::AddToStack {
                            ability,
                            valid_targets: ability.source(db)?.valid_targets(db)?,
                        });
                    }
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
                    pending.extend(Self::apply_action_result(
                        db,
                        all_players,
                        ActionResult::AddModifier { modifier },
                    )?);
                }
                UnresolvedActionResult::Mill {
                    count,
                    valid_targets,
                } => {
                    if valid_targets.len() == 1 {
                        pending.extend(Self::apply_action_result(
                            db,
                            all_players,
                            ActionResult::Mill {
                                count,
                                targets: valid_targets,
                            },
                        )?);
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
                        pending.extend(Self::apply_action_result(
                            db,
                            all_players,
                            ActionResult::ReturnFromGraveyardToLibrary {
                                targets: valid_targets,
                            },
                        )?);
                    } else {
                        let valid_targets =
                            compute_graveyard_targets(db, controller, source, &types)?;
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
                        pending.extend(Self::apply_action_result(
                            db,
                            all_players,
                            ActionResult::ReturnFromGraveyardToBattlefield {
                                targets: valid_targets,
                            },
                        )?);
                    } else {
                        let valid_targets =
                            compute_graveyard_targets(db, Controller::You, source, &types)?;
                        pending.push(UnresolvedActionResult::ReturnFromGraveyardToBattlefield {
                            source,
                            count,
                            types,
                            valid_targets,
                        })
                    }
                }
                UnresolvedActionResult::CreateToken { source, token } => {
                    pending.extend(Self::apply_action_result(
                        db,
                        all_players,
                        ActionResult::CreateToken { source, token },
                    )?);
                }
                UnresolvedActionResult::TutorLibrary {
                    source,
                    destination,
                    targets: _,
                    reveal,
                    restrictions,
                } => {
                    let targets = compute_deck_targets(db, source.controller(db)?, &restrictions)?;

                    pending.push(UnresolvedActionResult::TutorLibrary {
                        source,
                        destination,
                        targets,
                        reveal,
                        restrictions,
                    });
                }
            }
        }

        Ok(pending)
    }

    pub fn apply_action_results(
        db: &Connection,
        all_players: &mut AllPlayers,
        results: Vec<ActionResult>,
    ) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        let mut pending = vec![];

        for result in results {
            pending.extend(Self::apply_action_result(db, all_players, result)?);
        }

        Ok(pending)
    }

    fn apply_action_result(
        db: &Connection,
        all_players: &mut AllPlayers,
        result: ActionResult,
    ) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        match result {
            ActionResult::TapPermanent(card_id) => {
                card_id.tap(db)?;
            }
            ActionResult::PermanentToGraveyard(card_id) => {
                return Self::permanent_to_graveyard(db, card_id);
            }
            ActionResult::AddToStack { ability, targets } => {
                ability.move_to_stack(db, targets)?;
            }
            ActionResult::CloneCreatureNonTargeting { source, target } => {
                if let Some(target) = target {
                    target.clone_card(db, source)?;
                }
            }
            ActionResult::AddModifier { modifier } => {
                modifier.activate(db)?;
            }
            ActionResult::Mill { count, targets } => {
                for target in targets {
                    let deck = &mut all_players[target].deck;
                    for _ in 0..count {
                        let card_id = deck.draw();
                        if let Some(card_id) = card_id {
                            Self::library_to_graveyard(db, card_id)?;
                        }
                    }
                }
            }
            ActionResult::ReturnFromGraveyardToLibrary { targets } => {
                for target in targets {
                    target.move_to_library(db)?;
                    all_players[target.owner(db)?].deck.place_on_top(target);
                }
            }
            ActionResult::ReturnFromGraveyardToBattlefield { targets } => {
                let mut pending = vec![];
                for target in targets {
                    pending.extend(Self::add(db, target, vec![])?);
                }

                return Ok(pending);
            }
            ActionResult::CreateToken { source, token } => {
                let id = CardId::upload_token(db, source.controller(db)?, token)?;
                return Self::add(db, id, vec![]);
            }
        }

        Ok(vec![])
    }

    pub fn permanent_to_graveyard(
        db: &Connection,
        target: CardId,
    ) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        let mut pending = vec![];

        for triggerid in TriggerId::active_triggers_of_type(db, TriggerSource::PutIntoGraveyard)? {
            let trigger = triggerid.triggered_ability(db)?;
            if matches!(
                trigger.trigger.from,
                triggers::Location::Anywhere | triggers::Location::Battlefield
            ) && target.types_intersect(db, &trigger.trigger.for_types)?
            {
                for effect in trigger.effects {
                    match effect {
                        TriggeredEffect::CreateToken(token) => {
                            // TODO this should use the stack
                            pending.push(UnresolvedActionResult::CreateToken {
                                source: triggerid.listener(db)?,
                                token: token.clone(),
                            })
                        }
                    }
                }
            }
        }

        target.move_to_graveyard(db)?;

        Ok(pending)
    }

    pub fn library_to_graveyard(
        db: &Connection,
        target: CardId,
    ) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        let mut pending = vec![];

        for triggerid in TriggerId::active_triggers_of_type(db, TriggerSource::PutIntoGraveyard)? {
            let trigger = triggerid.triggered_ability(db)?;
            if matches!(
                trigger.trigger.from,
                triggers::Location::Anywhere | triggers::Location::Library
            ) && target.types_intersect(db, &trigger.trigger.for_types)?
            {
                for effect in trigger.effects {
                    match effect {
                        TriggeredEffect::CreateToken(token) => {
                            // TODO this should use the stack
                            pending.push(UnresolvedActionResult::CreateToken {
                                source: triggerid.listener(db)?,
                                token: token.clone(),
                            })
                        }
                    }
                }
            }
        }

        target.move_to_graveyard(db)?;

        Ok(pending)
    }

    pub fn stack_to_graveyard(
        db: &Connection,
        target: CardId,
    ) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        let mut pending = vec![];

        for triggerid in TriggerId::active_triggers_of_type(db, TriggerSource::PutIntoGraveyard)? {
            let trigger = triggerid.triggered_ability(db)?;
            if matches!(trigger.trigger.from, triggers::Location::Anywhere)
                && target.types_intersect(db, &trigger.trigger.for_types)?
            {
                for effect in trigger.effects {
                    match effect {
                        TriggeredEffect::CreateToken(token) => {
                            // TODO this should use the stack
                            pending.push(UnresolvedActionResult::CreateToken {
                                source: triggerid.listener(db)?,
                                token: token.clone(),
                            })
                        }
                    }
                }
            }
        }

        target.move_to_graveyard(db)?;

        Ok(pending)
    }

    pub fn exile(db: &Connection, target: CardId) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        target.move_to_exile(db)?;

        Ok(vec![])
    }
}

fn compute_deck_targets(
    db: &Connection,
    player: PlayerId,
    restrictions: &[Restriction],
) -> anyhow::Result<Vec<CardId>> {
    let mut results = vec![];

    for card in player.get_cards_in_zone(db, Location::Library)? {
        if !card.passes_restrictions(db, card, player, Controller::You, restrictions)? {
            continue;
        }

        results.push(card);
    }

    Ok(results)
}

fn compute_graveyard_targets(
    db: &Connection,
    controller: Controller,
    source_card: CardId,
    types: &HashSet<Type>,
) -> anyhow::Result<Vec<CardId>> {
    let targets = match controller {
        Controller::Any => AllPlayers::all_players(db)?,
        Controller::You => HashSet::from([source_card.controller(db)?]),
        Controller::Opponent => {
            // TODO this could probably be a query if I was smarter at sql
            let mut all = AllPlayers::all_players(db)?;
            all.remove(&source_card.controller(db)?);
            all
        }
    };
    let mut target_cards = vec![];

    for target in targets {
        let cards_in_graveyard: Vec<CardId> = target.get_cards_in_zone(db, Location::Graveyard)?;
        for card in cards_in_graveyard {
            if card.types_intersect(db, types)? {
                target_cards.push(card);
            }
        }
    }

    Ok(target_cards)
}
