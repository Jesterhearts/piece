use std::collections::{HashMap, HashSet};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    abilities::ActivatedAbilityEffect,
    battlefield::{Battlefield, UnresolvedActionResult},
    controller::Controller,
    effects::{spell, BattlefieldModifier, EffectDuration, GainMana, Token, TriggeredEffect},
    in_play::{AbilityId, CardId, Location, ModifierId, TriggerId},
    mana::Mana,
    player::{AllPlayers, PlayerId},
    targets::Restriction,
    types::Type,
};

#[derive(Debug, PartialEq)]
pub enum StackResult {
    AddToBattlefield(CardId),
    StackToGraveyard(CardId),
    ApplyToBattlefield(ModifierId),
    ApplyModifierToTarget {
        modifier: ModifierId,
        target: CardId,
    },
    ExileTarget(CardId),
    DamageTarget {
        quantity: usize,
        target: CardId,
    },
    ManifestTopOfLibrary(PlayerId),
    ModifyCreatures {
        targets: Vec<CardId>,
        modifier: ModifierId,
    },
    SpellCountered {
        id: Entry,
    },
    DrawCards {
        player: PlayerId,
        count: usize,
    },
    GainMana {
        player: PlayerId,
        mana: HashMap<Mana, usize>,
    },
    CreateToken {
        source: CardId,
        token: Token,
    },
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ActiveTarget {
    Stack { id: usize },
    Battlefield { id: CardId },
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Entry {
    Card(CardId),
    Ability(AbilityId),
    Trigger(TriggerId),
}

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub ty: Entry,
    pub targets: HashSet<ActiveTarget>,
    pub mode: Option<usize>,
}

#[derive(Debug)]
pub struct Stack;

impl Stack {
    pub fn split_second(db: &Connection) -> anyhow::Result<bool> {
        let mut query =
            db.prepare("SELECT NULL FROM cards WHERE location = (?1) AND split_second")?;
        let mut query = query.query((serde_json::to_string(&Location::Stack)?,))?;

        Ok(query.next()?.is_some())
    }

    pub fn target_nth(db: &Connection, nth: usize) -> anyhow::Result<ActiveTarget> {
        let mut cards_in_stack = db.prepare(
            "SELECT location_seq FROM cards WHERE location = (?1) ORDER BY location_seq ASC",
        )?;
        let mut abilities_in_stack = db.prepare(
            "SELECT stack_seq FROM abilities WHERE in_stack = TRUE ORDER BY stack_seq ASC",
        )?;
        let mut triggers_in_stack = db.prepare(
            "SELECT stack_seq FROM triggers WHERE in_stack = TRUE ORDER BY stack_seq ASC",
        )?;

        let mut cards_in_stack = cards_in_stack
            .query_map((serde_json::to_string(&Location::Stack)?,), |row| {
                row.get::<_, usize>(0)
            })?
            .map(|value| value.unwrap())
            .peekable();

        let mut abilities_in_stack = abilities_in_stack
            .query_map((), |row| row.get::<_, usize>(0))?
            .map(|value| value.unwrap())
            .peekable();

        let mut triggers_in_stack = triggers_in_stack
            .query_map((), |row| row.get::<_, usize>(0))?
            .map(|value| value.unwrap())
            .peekable();

        let mut target = 0;
        for _ in 0..=nth {
            let (max, index) = [
                (cards_in_stack.peek(), 0),
                (abilities_in_stack.peek(), 1),
                (triggers_in_stack.peek(), 2),
            ]
            .into_iter()
            .max_by_key(|(index, _)| index.copied().unwrap_or_default())
            .unwrap();

            target = target.max(max.copied().unwrap_or_default());

            match index {
                0 => {
                    cards_in_stack.next();
                }
                1 => {
                    abilities_in_stack.next();
                }
                2 => {
                    triggers_in_stack.next();
                }
                _ => unreachable!(),
            }
        }

        Ok(ActiveTarget::Stack { id: target })
    }

    pub fn in_stack(db: &Connection) -> anyhow::Result<HashMap<usize, Entry>> {
        let mut cards_in_stack =
            db.prepare("SELECT cardid, location_seq FROM cards WHERE location = (?1)")?;
        let mut abilities_in_stack =
            db.prepare("SELECT abilityid, stack_seq FROM abilities WHERE in_stack = TRUE")?;
        let mut triggers_in_stack =
            db.prepare("SELECT triggerid, stack_seq FROM triggers WHERE in_stack = TRUE")?;

        let mut cards_in_stack = cards_in_stack
            .query_map((serde_json::to_string(&Location::Stack)?,), |row| {
                Ok((row.get::<_, usize>(0)?, row.get::<_, usize>(1)?))
            })?
            .map(|value| value.unwrap())
            .peekable();

        let mut abilities_in_stack = abilities_in_stack
            .query_map((), |row| {
                Ok((row.get::<_, usize>(0)?, row.get::<_, usize>(1)?))
            })?
            .map(|value| value.unwrap())
            .peekable();

        let mut triggers_in_stack = triggers_in_stack
            .query_map((), |row| {
                Ok((row.get::<_, usize>(0)?, row.get::<_, usize>(1)?))
            })?
            .map(|value| value.unwrap())
            .peekable();

        let mut in_stack = HashMap::default();
        while cards_in_stack.peek().is_some()
            || abilities_in_stack.peek().is_some()
            || triggers_in_stack.peek().is_some()
        {
            let (max, index) = [
                (cards_in_stack.next(), 0),
                (abilities_in_stack.next(), 1),
                (triggers_in_stack.next(), 2),
            ]
            .into_iter()
            .max_by_key(|(index, _)| index.map(|(_, seq)| seq).unwrap_or_default())
            .unwrap();

            match index {
                0 => {
                    in_stack.insert(
                        max.map(|(_, seq)| seq).unwrap(),
                        max.map(|(id, _)| Entry::Card(id.into())).unwrap(),
                    );
                }
                1 => {
                    in_stack.insert(
                        max.map(|(_, seq)| seq).unwrap(),
                        max.map(|(id, _)| Entry::Ability(id.into())).unwrap(),
                    );
                }
                2 => {
                    in_stack.insert(
                        max.map(|(_, seq)| seq).unwrap(),
                        max.map(|(id, _)| Entry::Trigger(id.into())).unwrap(),
                    );
                }
                _ => unreachable!(),
            }
        }

        Ok(in_stack)
    }

    fn pop(db: &Connection) -> anyhow::Result<Option<StackEntry>> {
        let mut cards_in_stack = db.prepare(
            "SELECT cardid, targets, mode, location_seq FROM cards WHERE location = (?1) ORDER BY location_seq DESC",
        )?;
        let mut abilities_in_stack = db.prepare(
            "SELECT abilityid, targets, mode, stack_seq FROM abilities WHERE in_stack = TRUE ORDER BY stack_seq DESC",
        )?;

        let mut triggers_in_stack = db.prepare(
            "SELECT triggerid, targets, mode, stack_seq FROM triggers WHERE in_stack = TRUE ORDER BY stack_seq DESC",
        )?;

        let mut cards_in_stack = cards_in_stack
            .query_map((serde_json::to_string(&Location::Stack)?,), |row| {
                Ok((
                    row.get::<_, usize>(0)?,
                    serde_json::from_str::<HashSet<ActiveTarget>>(&row.get::<_, String>(1)?)
                        .unwrap(),
                    row.get::<_, Option<usize>>(2)?,
                    row.get::<_, usize>(3)?,
                ))
            })?
            .map(|value| value.unwrap());

        let mut abilities_in_stack = abilities_in_stack
            .query_map((), |row| {
                Ok((
                    row.get::<_, usize>(0)?,
                    serde_json::from_str::<HashSet<ActiveTarget>>(&row.get::<_, String>(1)?)
                        .unwrap(),
                    row.get::<_, Option<usize>>(2)?,
                    row.get::<_, usize>(3)?,
                ))
            })?
            .map(|value| value.unwrap());

        let mut triggers_in_stack = triggers_in_stack
            .query_map((), |row| {
                Ok((
                    row.get::<_, usize>(0)?,
                    serde_json::from_str::<HashSet<ActiveTarget>>(&row.get::<_, String>(1)?)
                        .unwrap(),
                    row.get::<_, Option<usize>>(2)?,
                    row.get::<_, usize>(3)?,
                ))
            })?
            .map(|value| value.unwrap());

        let (max, index) = [
            (cards_in_stack.next(), 0),
            (abilities_in_stack.next(), 1),
            (triggers_in_stack.next(), 2),
        ]
        .into_iter()
        .max_by_key(|(index, _)| {
            index
                .as_ref()
                .map(|(_, _, _, seq)| *seq)
                .unwrap_or_default()
        })
        .unwrap();

        match index {
            0 => Ok(max.map(|(id, targets, mode, _)| StackEntry {
                ty: Entry::Card(CardId::from(id)),
                targets,
                mode,
            })),
            1 => Ok(max.map(|(id, targets, mode, _)| StackEntry {
                ty: Entry::Ability(AbilityId::from(id)),
                targets,
                mode,
            })),
            2 => Ok(max.map(|(id, targets, mode, _)| StackEntry {
                ty: Entry::Trigger(TriggerId::from(id)),
                targets,
                mode,
            })),
            _ => unreachable!(),
        }
    }

    pub fn is_empty(db: &Connection) -> anyhow::Result<bool> {
        let mut cards_in_stack =
            db.prepare("SELECT NULL FROM cards WHERE location = (?1) ORDER BY location_seq ASC")?;
        let mut abilities_in_stack =
            db.prepare("SELECT NULL FROM abilities WHERE in_stack = TRUE ORDER BY stack_seq ASC")?;

        let mut triggers_in_stack =
            db.prepare("SELECT NULL FROM triggers WHERE in_stack = TRUE ORDER BY stack_seq ASC")?;

        Ok(cards_in_stack
            .query((serde_json::to_string(&Location::Stack)?,))?
            .next()?
            .is_none()
            && abilities_in_stack.query(())?.next()?.is_none()
            && triggers_in_stack.query(())?.next()?.is_none())
    }

    pub fn resolve_1(db: &Connection) -> anyhow::Result<Vec<StackResult>> {
        let Some(next) = Self::pop(db)? else {
            return Ok(vec![]);
        };

        let in_stack = Self::in_stack(db)?;

        let mut result = vec![];

        match next.ty {
            Entry::Card(resolving_card) => {
                for effect in resolving_card.effects(db)? {
                    let effect = if effect.threshold.is_some()
                        && Battlefield::number_of_cards_in_graveyard(
                            db,
                            resolving_card.controller(db)?,
                        )? >= 7
                    {
                        effect.threshold.unwrap()
                    } else {
                        effect.effect
                    };

                    match effect {
                        spell::Effect::CounterSpell {
                            target: restrictions,
                        } => {
                            if next.targets.is_empty() {
                                return Ok(vec![StackResult::StackToGraveyard(resolving_card)]);
                            }

                            for target in next.targets.iter() {
                                match target {
                                    ActiveTarget::Stack { id } => {
                                        let Some(maybe_target) = in_stack.get(id) else {
                                            // Spell has left the stack already
                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        };

                                        match maybe_target {
                                            Entry::Card(maybe_target) => {
                                                if !maybe_target.can_be_countered(
                                                    db,
                                                    resolving_card.controller(db)?,
                                                    &restrictions,
                                                )? {
                                                    // Spell is no longer a valid target.
                                                    return Ok(vec![
                                                        StackResult::StackToGraveyard(
                                                            resolving_card,
                                                        ),
                                                    ]);
                                                }
                                            }
                                            Entry::Ability(_) => {
                                                // Vanilla counterspells can't counter activated abilities.
                                                return Ok(vec![StackResult::StackToGraveyard(
                                                    resolving_card,
                                                )]);
                                            }
                                            Entry::Trigger(_) => {
                                                // Vanilla counterspells can't counter triggered abilities.
                                                return Ok(vec![StackResult::StackToGraveyard(
                                                    resolving_card,
                                                )]);
                                            }
                                        }

                                        // If we reach here, we know the spell can be countered.
                                        result.push(StackResult::SpellCountered {
                                            id: *maybe_target,
                                        });
                                    }
                                    ActiveTarget::Battlefield { .. } => {
                                        // Cards on the battlefield aren't valid targets of counterspells
                                        return Ok(vec![StackResult::StackToGraveyard(
                                            resolving_card,
                                        )]);
                                    }
                                }
                            }
                        }
                        spell::Effect::GainMana { mana } => {
                            if !gain_mana(
                                resolving_card.controller(db)?,
                                &mana,
                                next.mode,
                                &mut result,
                            ) {
                                return Ok(vec![StackResult::StackToGraveyard(resolving_card)]);
                            }
                        }
                        spell::Effect::BattlefieldModifier(modifier) => {
                            result.push(StackResult::ApplyToBattlefield(
                                ModifierId::upload_single_modifier(
                                    db,
                                    resolving_card,
                                    &modifier,
                                    true,
                                )?,
                            ));
                        }
                        spell::Effect::ControllerDrawCards(count) => {
                            result.push(StackResult::DrawCards {
                                player: resolving_card.controller(db)?,
                                count,
                            });
                        }
                        spell::Effect::ModifyCreature(modifier) => {
                            let modifier = ModifierId::upload_single_modifier(
                                db,
                                resolving_card,
                                &modifier,
                                true,
                            )?;

                            let mut targets = vec![];
                            for target in next.targets.iter() {
                                match target {
                                    ActiveTarget::Stack { .. } => {
                                        // Stack is not a valid target.
                                        return Ok(vec![StackResult::StackToGraveyard(
                                            resolving_card,
                                        )]);
                                    }
                                    ActiveTarget::Battlefield { id } => {
                                        targets.push(id);
                                    }
                                }
                            }

                            for target in targets {
                                target.apply_modifier(db, modifier)?;
                            }
                        }
                        spell::Effect::ExileTargetCreature => {
                            for target in next.targets.iter() {
                                match target {
                                    ActiveTarget::Stack { .. } => {
                                        return Ok(vec![StackResult::StackToGraveyard(
                                            resolving_card,
                                        )]);
                                    }
                                    ActiveTarget::Battlefield { id } => {
                                        if !id.is_in_location(db, Location::Battlefield)? {
                                            // Permanent no longer on battlefield.
                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        }

                                        if !id
                                            .can_be_targeted(db, resolving_card.controller(db)?)?
                                        {
                                            // Card is no longer a valid target.
                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        }

                                        if !id
                                            .types_intersect(db, &HashSet::from([Type::Creature]))?
                                        {
                                            // Target isn't a creature

                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        }

                                        result.push(StackResult::ExileTarget(*id));
                                    }
                                }
                            }
                        }
                        spell::Effect::ExileTargetCreatureManifestTopOfLibrary => {
                            for target in next.targets.iter() {
                                match target {
                                    ActiveTarget::Stack { .. } => {
                                        return Ok(vec![StackResult::StackToGraveyard(
                                            resolving_card,
                                        )]);
                                    }
                                    ActiveTarget::Battlefield { id } => {
                                        if !id.is_in_location(db, Location::Battlefield)? {
                                            // Permanent no longer on battlefield.
                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        }

                                        if !id
                                            .can_be_targeted(db, resolving_card.controller(db)?)?
                                        {
                                            // Card is no longer a valid target.
                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        }

                                        if !id
                                            .types_intersect(db, &HashSet::from([Type::Creature]))?
                                        {
                                            // Target isn't a creature
                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        }

                                        result.push(StackResult::ExileTarget(*id));
                                        result.push(StackResult::ManifestTopOfLibrary(
                                            id.controller(db)?,
                                        ));
                                    }
                                }
                            }
                        }
                        spell::Effect::DealDamage(dmg) => {
                            for target in next.targets.iter() {
                                match target {
                                    ActiveTarget::Stack { .. } => {
                                        return Ok(vec![StackResult::StackToGraveyard(
                                            resolving_card,
                                        )]);
                                    }
                                    ActiveTarget::Battlefield { id } => {
                                        if !id.is_in_location(db, Location::Battlefield)? {
                                            // Permanent no longer on battlefield.
                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        }

                                        if !id
                                            .can_be_targeted(db, resolving_card.controller(db)?)?
                                        {
                                            // Card is no longer a valid target.
                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        }

                                        if !id.passes_restrictions(
                                            db,
                                            resolving_card,
                                            resolving_card.controller(db)?,
                                            Controller::Any,
                                            &dmg.restrictions,
                                        )? {
                                            return Ok(vec![StackResult::StackToGraveyard(
                                                resolving_card,
                                            )]);
                                        }

                                        result.push(StackResult::DamageTarget {
                                            quantity: dmg.quantity,
                                            target: *id,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                if resolving_card.is_permanent(db)? {
                    result.push(StackResult::AddToBattlefield(resolving_card));
                } else {
                    result.push(StackResult::StackToGraveyard(resolving_card));
                }

                Ok(result)
            }
            Entry::Ability(ability) => {
                for effect in ability.effects(db)? {
                    match effect {
                        ActivatedAbilityEffect::CounterSpell { target: _ } => todo!(),
                        ActivatedAbilityEffect::GainMana { mana } => {
                            if !gain_mana(ability.controller(db)?, &mana, next.mode, &mut result) {
                                return Ok(vec![]);
                            }
                        }
                        ActivatedAbilityEffect::BattlefieldModifier(modifier) => {
                            let modifier = ModifierId::upload_single_modifier(
                                db,
                                ability.source(db)?,
                                &modifier,
                                true,
                            )?;
                            if ability.apply_to_self(db)? {
                                result.push(StackResult::ApplyModifierToTarget {
                                    modifier,
                                    target: ability.source(db)?,
                                });
                            } else {
                                result.push(StackResult::ApplyToBattlefield(modifier));
                            }
                        }
                        ActivatedAbilityEffect::ControllerDrawCards(count) => {
                            result.push(StackResult::DrawCards {
                                player: ability.controller(db)?,
                                count,
                            });
                        }
                        ActivatedAbilityEffect::Equip(modifiers) => {
                            if next.targets.is_empty() {
                                // Effect fizzles due to lack of target.
                                return Ok(vec![]);
                            }

                            assert_eq!(next.targets.len(), 1);

                            match next.targets.iter().next().unwrap() {
                                ActiveTarget::Stack { .. } => {
                                    // Can't equip things on the stack
                                    return Ok(vec![]);
                                }
                                ActiveTarget::Battlefield { id } => {
                                    if !id.can_be_targeted(db, ability.controller(db)?)? {
                                        // Card is not a valid target, spell fizzles.
                                        return Ok(vec![]);
                                    }

                                    for modifier in modifiers {
                                        let modifier = ModifierId::upload_single_modifier(
                                            db,
                                            ability.source(db)?,
                                            &BattlefieldModifier {
                                                modifier,
                                                controller: Controller::You,
                                                duration:
                                                    EffectDuration::UntilSourceLeavesBattlefield,
                                                restrictions: vec![Restriction::OfType {
                                                    types: HashSet::from([Type::Creature]),
                                                    subtypes: Default::default(),
                                                }],
                                            },
                                            true,
                                        )?;

                                        result.push(StackResult::ModifyCreatures {
                                            targets: vec![*id],
                                            modifier,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(result)
            }
            Entry::Trigger(trigger) => {
                for effect in trigger.effects(db)? {
                    match effect {
                        TriggeredEffect::CreateToken(token) => {
                            result.push(StackResult::CreateToken {
                                source: trigger.listener(db)?,
                                token,
                            });
                        }
                    }
                }
                Ok(result)
            }
        }
    }

    pub fn apply_results(
        db: &Connection,
        all_players: &mut AllPlayers,
        results: Vec<StackResult>,
    ) -> anyhow::Result<Vec<UnresolvedActionResult>> {
        let mut pending = vec![];

        for result in results {
            match result {
                StackResult::AddToBattlefield(card) => {
                    pending.extend(Battlefield::add_from_stack(db, card, vec![])?);
                }
                StackResult::ApplyToBattlefield(modifier) => {
                    modifier.activate(db)?;
                }
                StackResult::ExileTarget(target) => {
                    pending.extend(Battlefield::exile(db, target)?);
                }
                StackResult::ManifestTopOfLibrary(player) => {
                    pending.extend(all_players[player].manifest(db)?);
                }
                StackResult::ModifyCreatures { targets, modifier } => {
                    for target in targets {
                        target.apply_modifier(db, modifier)?;
                    }
                }
                StackResult::SpellCountered { id } => match id {
                    Entry::Card(card) => {
                        pending.extend(Battlefield::stack_to_graveyard(db, card)?);
                    }
                    Entry::Ability(_) | Entry::Trigger(_) => unreachable!(),
                },
                StackResult::DrawCards { player, count } => {
                    all_players[player].draw(db, count)?;
                }
                StackResult::GainMana { player, mana } => {
                    for (mana, count) in mana {
                        for _ in 0..count {
                            all_players[player].mana_pool.apply(mana);
                        }
                    }
                }
                StackResult::StackToGraveyard(card) => {
                    pending.extend(Battlefield::stack_to_graveyard(db, card)?);
                }
                StackResult::ApplyModifierToTarget { modifier, target } => {
                    target.apply_modifier(db, modifier)?;
                }
                StackResult::CreateToken { source, token } => {
                    let id = CardId::upload_token(db, source.controller(db)?, token)?;
                    pending.extend(Battlefield::add_from_stack(db, id, vec![])?);
                }
                StackResult::DamageTarget { quantity, target } => {
                    target.mark_damage(db, quantity)?;
                }
            }
        }

        Ok(pending)
    }
}

fn gain_mana(
    controller: PlayerId,
    mana: &GainMana,
    mode: Option<usize>,
    result: &mut Vec<StackResult>,
) -> bool {
    let mut manas = HashMap::default();
    match mana {
        GainMana::Specific { gains } => {
            for gain in gains.iter() {
                *manas.entry(*gain).or_default() += 1;
            }
        }
        GainMana::Choice { choices } => {
            let Some(mode) = mode else {
                // No mode selected for modal ability.
                return false;
            };

            for gain in choices[mode].iter() {
                *manas.entry(*gain).or_default() += 1;
            }
        }
    };

    result.push(StackResult::GainMana {
        player: controller,
        mana: manas,
    });

    true
}

#[cfg(test)]
mod tests {
    use crate::{
        in_play::CardId,
        load_cards,
        player::AllPlayers,
        prepare_db,
        stack::{Stack, StackResult},
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let db = prepare_db()?;
        let mut all_players = AllPlayers::default();
        let player = all_players.new_player();
        let card1 = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;

        card1.move_to_stack(&db, Default::default())?;

        let results = Stack::resolve_1(&db)?;

        assert_eq!(results, [StackResult::AddToBattlefield(card1)]);
        Stack::apply_results(&db, &mut all_players, results)?;

        assert!(Stack::is_empty(&db)?);

        Ok(())
    }
}
