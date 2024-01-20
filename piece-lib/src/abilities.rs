use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    effects::EffectBehaviors,
    in_play::Database,
    log::LogId,
    pending_results::PendingResults,
    player::{mana_pool::SpendReason, Owner},
    protogen::{
        cost::{
            ability_restriction,
            additional_cost::{self, ExileXOrMoreCards, RemoveCounters},
            AbilityCost,
        },
        counters::Counter,
        effects::{static_ability, ActivatedAbility, Effect, GainManaAbility},
        ids::{ActivatedAbilityId, CardId, GainManaAbilityId},
    },
    turns::Phase,
};

impl ActivatedAbility {
    pub(crate) fn can_be_played_from_hand(&self) -> bool {
        self.effects
            .iter()
            .any(|effect| effect.effect.as_ref().unwrap().cycling())
    }

    pub(crate) fn can_be_activated(
        &self,
        db: &Database,
        source: &CardId,
        id: &Ability,
        activator: crate::player::Owner,
        pending: &Option<PendingResults>,
    ) -> bool {
        let banned = db[source].modified_static_abilities.iter().any(|ability| {
            matches!(
                db[ability].ability,
                static_ability::Ability::PreventAbilityActivation(_)
            )
        });

        if banned {
            return false;
        }

        let in_battlefield = db.battlefield[db[source].controller].contains(source);

        if pending.is_some() && !pending.as_ref().unwrap().is_empty() {
            return false;
        }

        let in_hand = db.hand[activator].contains(source);
        if in_hand && !self.can_be_played_from_hand() {
            return false;
        }

        if self.can_be_played_from_hand() && !in_hand {
            return false;
        }

        if !in_battlefield {
            return false;
        }

        // TODO: Effects like Xantcha
        let controller = db[source].controller;
        if controller != activator {
            return false;
        }

        let is_sorcery = self.sorcery_speed
            || self
                .effects
                .iter()
                .any(|effect| effect.effect.as_ref().unwrap().is_sorcery_speed());
        if is_sorcery {
            if controller != db.turn.active_player() {
                return false;
            }

            if !matches!(
                db.turn.phase,
                Phase::PreCombatMainPhase | Phase::PostCombatMainPhase
            ) {
                return false;
            }

            if !db.stack.is_empty() {
                return false;
            }
        }

        if !can_pay_costs(db, id, self.cost.get_or_default(), source) {
            return false;
        }

        true
    }
}

impl GainManaAbility {
    fn can_be_activated(
        &self,
        db: &Database,
        id: &Ability,
        source: &CardId,
        activator: Owner,
    ) -> bool {
        if !db.battlefield[db[source].controller].contains(source) {
            return false;
        }

        if db[source].controller != activator {
            return false;
        }

        can_pay_costs(db, id, &self.cost, source)
    }
}

#[derive(Debug, Clone)]
pub enum Ability {
    Activated(ActivatedAbilityId),
    Mana(GainManaAbilityId),
    EtbOrTriggered(Vec<Effect>),
}

impl Ability {
    pub(crate) fn cost<'db>(&self, db: &'db Database) -> Option<&'db AbilityCost> {
        match self {
            Ability::Activated(id) => Some(&db[id].ability.cost),
            Ability::Mana(id) => Some(&db[id].ability.cost),
            Ability::EtbOrTriggered(_) => None,
        }
    }

    pub(crate) fn effects(&self, db: &Database) -> Vec<Effect> {
        match self {
            Ability::Activated(id) => db[id].ability.effects.clone(),
            Ability::Mana(_) => vec![],
            Ability::EtbOrTriggered(effects) => effects.clone(),
        }
    }

    pub fn text(&self, db: &Database) -> String {
        match self {
            Ability::Activated(id) => db[id].ability.oracle_text.clone(),
            Ability::Mana(id) => db[id].ability.oracle_text.clone(),
            Ability::EtbOrTriggered(effects) => {
                effects.iter().map(|effect| &effect.oracle_text).join("\n")
            }
        }
    }

    pub fn can_be_activated(
        &self,
        db: &Database,
        source: &CardId,
        activator: crate::player::Owner,
        pending: &Option<PendingResults>,
    ) -> bool {
        match self {
            Ability::Activated(activated) => {
                if !db[activated]
                    .ability
                    .can_be_activated(db, source, self, activator, pending)
                {
                    return false;
                }

                let targets = source.targets_for_ability(db, self, &HashSet::default());

                db[activated]
                    .ability
                    .effects
                    .iter()
                    .map(|effect| effect.effect.as_ref().unwrap().needs_targets(db, source))
                    .zip(targets)
                    .all(|(needs, has)| has.len() >= needs)
            }
            Ability::Mana(id) => db[id].ability.can_be_activated(db, self, source, activator),
            Ability::EtbOrTriggered(_) => false,
        }
    }
}

pub(crate) fn can_pay_costs(
    db: &Database,
    id: &Ability,
    cost: &AbilityCost,
    source: &CardId,
) -> bool {
    if cost.tap && (db[source].tapped || source.summoning_sick(db)) {
        return false;
    }
    let controller = db[source].controller;

    for cost in cost.additional_costs.iter() {
        match cost.cost.as_ref().unwrap() {
            additional_cost::Cost::SacrificeSource(_) => {
                if !source.can_be_sacrificed(db) {
                    return false;
                }
            }
            additional_cost::Cost::PayLife(life) => {
                if db.all_players[controller].life_total <= life.count as i32 {
                    return false;
                }
            }
            additional_cost::Cost::SacrificePermanent(sac) => {
                let any_target = db.battlefield[controller].iter().any(|card| {
                    card.passes_restrictions(db, LogId::current(db), source, &sac.restrictions)
                });
                if !any_target {
                    return false;
                }
            }
            additional_cost::Cost::TapPermanent(tap) => {
                let any_target = db.battlefield[controller].iter().any(|card| {
                    card.passes_restrictions(db, LogId::current(db), source, &tap.restrictions)
                });
                if !any_target {
                    return false;
                }
            }
            additional_cost::Cost::ExileCard(exile) => {
                let any_target = db.battlefield[controller].iter().any(|card| {
                    card.passes_restrictions(db, LogId::current(db), source, &exile.restrictions)
                });
                if !any_target {
                    return false;
                }
            }
            additional_cost::Cost::ExileXOrMoreCards(ExileXOrMoreCards {
                minimum,
                restrictions,
                ..
            }) => {
                let targets = db.battlefield[controller]
                    .iter()
                    .filter(|card| {
                        card.passes_restrictions(db, LogId::current(db), source, restrictions)
                    })
                    .count() as u32;

                if targets < *minimum {
                    return false;
                }
            }
            additional_cost::Cost::DiscardThis(_) => {
                if !db.hand[controller].contains(source) {
                    return false;
                }
            }
            additional_cost::Cost::RemoveCounters(RemoveCounters { counter, count, .. }) => {
                let counters = if let Counter::ANY = counter.enum_value().unwrap() {
                    db[source].counters.values().sum::<usize>()
                } else {
                    db[source]
                        .counters
                        .get(&counter.enum_value().unwrap())
                        .copied()
                        .unwrap_or_default()
                };

                if counters < *count as usize {
                    return false;
                }
            }

            // These are too complicated to compute, so just give up. The user can cancel if they can't actually pay.
            additional_cost::Cost::ExileCardsCmcX(_) => {}
            additional_cost::Cost::ExileSharingCardType { .. } => {}
            additional_cost::Cost::TapPermanentsPowerXOrMore { .. } => {}
        }
    }

    for restriction in cost.restrictions.iter() {
        match restriction.restriction.as_ref().unwrap() {
            ability_restriction::Restriction::AttackedWithXOrMoreCreatures(x) => {
                if db.turn.number_of_attackers_this_turn < x.x_is as usize {
                    return false;
                }
            }
            ability_restriction::Restriction::OncePerTurn(_) => match id {
                Ability::Activated(id) => {
                    if db.turn.activated_abilities.contains(id) {
                        return false;
                    }
                }
                Ability::Mana(_) => todo!(),
                Ability::EtbOrTriggered(_) => todo!(),
            },
        }
    }

    if !db.all_players[controller].can_meet_cost(
        db,
        &cost.mana_cost,
        &[],
        &SpendReason::Activating(source.clone()),
    ) {
        return false;
    }

    true
}
