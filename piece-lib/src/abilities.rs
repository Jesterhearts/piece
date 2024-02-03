use crate::{
    effects::PendingEffects,
    in_play::{ActivatedAbilityId, CardId, Database, GainManaAbilityId},
    player::Owner,
    protogen::{
        cost::{ability_restriction, AbilityCost},
        effects::{
            static_ability, ActivatedAbility, Effect, EtbAbility, GainManaAbility, TargetSelection,
            TriggeredAbility,
        },
    },
    turns::Phase,
};

impl ActivatedAbility {
    pub(crate) fn can_be_played_from_hand(&self) -> bool {
        self.can_activate_in_hand
    }

    pub(crate) fn can_be_activated(
        &self,
        db: &Database,
        source: CardId,
        id: &Ability,
        activator: crate::player::Owner,
        pending: &Option<PendingEffects>,
    ) -> bool {
        let banned = db[source].modified_static_abilities.iter().any(|ability| {
            matches!(
                db[*ability].ability,
                static_ability::Ability::PreventAbilityActivation(_)
            )
        });

        if banned {
            return false;
        }

        let in_battlefield = db.battlefield[db[source].controller].contains(&source);

        if pending.is_some() && !pending.as_ref().unwrap().is_empty() {
            return false;
        }

        let in_hand = db.hand[activator].contains(&source);
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

        if self.sorcery_speed {
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

        if !passes_restrictions(db, id, self.cost.get_or_default(), source) {
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
        source: CardId,
        activator: Owner,
    ) -> bool {
        if !db.battlefield[db[source].controller].contains(&source) {
            return false;
        }

        if db[source].controller != activator {
            return false;
        }

        passes_restrictions(db, id, &self.cost, source)
    }
}

#[derive(Debug, Clone)]
pub enum Ability {
    Activated(ActivatedAbilityId),
    Mana(GainManaAbilityId),
    Etb(EtbAbility),
    TriggeredAbility(TriggeredAbility),
}

impl Ability {
    pub(crate) fn cost<'db>(&self, db: &'db Database) -> Option<&'db AbilityCost> {
        match self {
            Ability::Activated(id) => db[*id].ability.cost.as_ref(),
            Ability::Mana(id) => db[*id].ability.cost.as_ref(),
            Ability::Etb(_) | Ability::TriggeredAbility(_) => None,
        }
    }

    pub(crate) fn targets<'db>(&'db self, db: &'db Database) -> Option<&'db TargetSelection> {
        match self {
            Ability::Activated(id) => db[*id].ability.targets.as_ref(),
            Ability::Mana(_) => None,
            Ability::Etb(etb) => etb.targets.as_ref(),
            Ability::TriggeredAbility(triggered) => triggered.targets.as_ref(),
        }
    }

    pub(crate) fn additional_costs<'db>(&self, db: &'db Database) -> Option<&'db [Effect]> {
        match self {
            Ability::Activated(id) => Some(&db[*id].ability.additional_costs),
            Ability::Mana(id) => Some(&db[*id].ability.additional_costs),
            Ability::Etb(_) | Ability::TriggeredAbility(_) => None,
        }
    }

    pub(crate) fn effects(&self, db: &Database) -> Vec<Effect> {
        match self {
            Ability::Activated(id) => db[*id].ability.effects.clone(),
            Ability::Mana(id) => db[*id].ability.effects.clone(),
            Ability::Etb(etb) => etb.effects.clone(),
            Ability::TriggeredAbility(triggered) => triggered.effects.clone(),
        }
    }

    pub fn text(&self, db: &Database) -> String {
        match self {
            Ability::Activated(id) => db[*id].ability.oracle_text.clone(),
            Ability::Mana(id) => db[*id].ability.oracle_text.clone(),
            Ability::Etb(etb) => etb.oracle_text.clone(),
            Ability::TriggeredAbility(triggered) => triggered.oracle_text.clone(),
        }
    }

    pub fn can_be_activated(
        &self,
        db: &Database,
        source: CardId,
        activator: crate::player::Owner,
        pending: &Option<PendingEffects>,
    ) -> bool {
        match self {
            Ability::Activated(activated) => {
                if !db[*activated]
                    .ability
                    .can_be_activated(db, source, self, activator, pending)
                {
                    return false;
                }

                true
            }
            Ability::Mana(id) => db[*id]
                .ability
                .can_be_activated(db, self, source, activator),
            _ => false,
        }
    }

    pub(crate) fn is_craft(&self, db: &Database) -> bool {
        match self {
            Ability::Activated(id) => db[*id].ability.craft,
            _ => false,
        }
    }
}

pub(crate) fn passes_restrictions(
    db: &Database,
    id: &Ability,
    cost: &AbilityCost,
    source: CardId,
) -> bool {
    if cost.tap && (db[source].tapped || source.summoning_sick(db)) {
        return false;
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
                _ => return false,
            },
        }
    }

    true
}
