use std::{cell::OnceCell, collections::HashMap, rc::Rc, sync::atomic::Ordering};

use bevy_ecs::{component::Component, entity::Entity};
use derive_more::From;

use crate::{
    abilities::{Ability, ActivatedAbility, ApplyToSelf, GainMana, GainManaAbility},
    card::OracleText,
    cost::{AbilityCost, AdditionalCost},
    effects::{AnyEffect, Effects},
    in_play::{CardId, Database, InStack, OnBattlefield, NEXT_STACK_SEQ},
    mana::Mana,
    player::{AllPlayers, Controller},
    stack::{ActiveTarget, Settled, Stack, Targets},
    turns::{Phase, Turn},
    types::Subtype,
};

pub type MakeLandAbility = Rc<dyn Fn(&mut Database, CardId) -> AbilityId>;

thread_local! {
    static INIT_LAND_ABILITIES: OnceCell<HashMap<Subtype, MakeLandAbility>> = OnceCell::new();
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Component)]
pub struct AbilityId(Entity);

impl AbilityId {
    pub fn upload_ability(db: &mut Database, cardid: CardId, ability: Ability) -> AbilityId {
        match ability {
            Ability::Activated(ability) => {
                let mut entity =
                    db.abilities
                        .spawn((cardid, ability.cost, Effects(ability.effects)));

                if ability.apply_to_self {
                    entity.insert(ApplyToSelf);
                }

                if !ability.oracle_text.is_empty() {
                    entity.insert(OracleText(ability.oracle_text.clone()));
                }

                Self(entity.id())
            }
            Ability::Mana(ability) => {
                let entity = db.abilities.spawn((cardid, ability.cost, ability.gain));

                Self(entity.id())
            }
            Ability::ETB {
                effects,
                oracle_text,
            } => {
                let mut entity = db.abilities.spawn((cardid, Effects(effects)));
                if let Some(text) = oracle_text.as_ref() {
                    entity.insert(OracleText(text.clone()));
                }

                debug!("Uploaded {:?}", entity.id());
                Self(entity.id())
            }
        }
    }

    pub fn land_abilities() -> HashMap<Subtype, MakeLandAbility> {
        INIT_LAND_ABILITIES.with(|init| {
            init.get_or_init(|| {
                let mut abilities: HashMap<Subtype, MakeLandAbility> = HashMap::new();

                let add = Rc::new(|db: &mut Database, source| {
                    AbilityId(
                        db.abilities
                            .spawn((
                                AbilityCost {
                                    mana_cost: vec![],
                                    tap: true,
                                    additional_cost: vec![],
                                },
                                GainMana::Specific {
                                    gains: vec![Mana::White],
                                },
                                source,
                            ))
                            .id(),
                    )
                });
                abilities.insert(Subtype::Plains, add);

                let add = Rc::new(|db: &mut Database, source| {
                    AbilityId(
                        db.abilities
                            .spawn((
                                AbilityCost {
                                    mana_cost: vec![],
                                    tap: true,
                                    additional_cost: vec![],
                                },
                                GainMana::Specific {
                                    gains: vec![Mana::Blue],
                                },
                                source,
                            ))
                            .id(),
                    )
                });
                abilities.insert(Subtype::Island, add);

                let add = Rc::new(|db: &mut Database, source| {
                    AbilityId(
                        db.abilities
                            .spawn((
                                AbilityCost {
                                    mana_cost: vec![],
                                    tap: true,
                                    additional_cost: vec![],
                                },
                                GainMana::Specific {
                                    gains: vec![Mana::Black],
                                },
                                source,
                            ))
                            .id(),
                    )
                });
                abilities.insert(Subtype::Swamp, add);

                let add = Rc::new(|db: &mut Database, source| {
                    AbilityId(
                        db.abilities
                            .spawn((
                                AbilityCost {
                                    mana_cost: vec![],
                                    tap: true,
                                    additional_cost: vec![],
                                },
                                GainMana::Specific {
                                    gains: vec![Mana::Red],
                                },
                                source,
                            ))
                            .id(),
                    )
                });
                abilities.insert(Subtype::Mountain, add);

                let add = Rc::new(|db: &mut Database, source| {
                    AbilityId(
                        db.abilities
                            .spawn((
                                AbilityCost {
                                    mana_cost: vec![],
                                    tap: true,
                                    additional_cost: vec![],
                                },
                                GainMana::Specific {
                                    gains: vec![Mana::Green],
                                },
                                source,
                            ))
                            .id(),
                    )
                });
                abilities.insert(Subtype::Forest, add);

                abilities
            })
            .clone()
        })
    }

    pub fn update_stack_seq(self, db: &mut Database) {
        *db.abilities.get_mut::<InStack>(self.0).unwrap() =
            InStack(NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed));
    }

    pub fn move_to_stack(self, db: &mut Database, source: CardId, targets: Vec<ActiveTarget>) {
        if Stack::split_second(db) {
            return;
        }

        db.abilities.spawn((
            self,
            InStack(NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed)),
            Targets(targets),
            source,
        ));
    }

    pub fn remove_from_stack(self, db: &mut Database) {
        db.abilities.despawn(self.0);
    }

    pub fn original(self, db: &Database) -> AbilityId {
        db.abilities
            .get::<AbilityId>(self.0)
            .copied()
            .unwrap_or(self)
    }

    pub fn ability(self, db: &mut Database) -> Ability {
        let this = self.original(db);

        if let Some((cost, effects, text, apply_to_self)) = db
            .abilities
            .query::<(
                Entity,
                &AbilityCost,
                &Effects,
                Option<&OracleText>,
                Option<&ApplyToSelf>,
            )>()
            .iter(&db.abilities)
            .filter_map(|(e, cost, effect, text, apply_to_self)| {
                if Self(e) == this {
                    Some((cost, effect, text, apply_to_self))
                } else {
                    None
                }
            })
            .next()
        {
            Ability::Activated(ActivatedAbility {
                cost: cost.clone(),
                effects: effects.0.clone(),
                apply_to_self: apply_to_self.is_some(),
                oracle_text: text.map(|t| t.0.clone()).unwrap_or_default(),
            })
        } else if let Some((effects, text)) = db
            .abilities
            .query::<(Entity, &Effects, Option<&OracleText>)>()
            .iter(&db.abilities)
            .filter_map(|(e, effects, text)| {
                if Self(e) == this {
                    Some((effects, text))
                } else {
                    None
                }
            })
            .next()
        {
            Ability::ETB {
                effects: effects.0.clone(),
                oracle_text: text.map(|t| t.0.clone()),
            }
        } else {
            Ability::Mana(this.gain_mana_ability(db))
        }
    }

    pub fn gain_mana_ability(self, db: &mut Database) -> GainManaAbility {
        let (cost, gain) = db
            .abilities
            .query::<(Entity, &AbilityCost, &GainMana)>()
            .iter(&db.abilities)
            .filter_map(|(e, cost, effect)| {
                if Self(e) == self {
                    Some((cost, effect))
                } else {
                    None
                }
            })
            .next()
            .unwrap();

        GainManaAbility {
            cost: cost.clone(),
            gain: gain.clone(),
        }
    }

    pub fn text(self, db: &mut Database) -> String {
        match self.ability(db) {
            Ability::Activated(activated) => {
                format!("{}: {}", activated.cost.text(), activated.oracle_text)
            }
            Ability::Mana(ability) => ability.text(),
            Ability::ETB { oracle_text, .. } => oracle_text.unwrap_or_else(|| "ETB".to_owned()),
        }
    }

    pub fn apply_to_self(self, db: &mut Database) -> bool {
        db.abilities
            .get::<ApplyToSelf>(self.original(db).0)
            .is_some()
    }

    pub fn effects(self, db: &mut Database) -> Vec<AnyEffect> {
        db.abilities
            .get::<Effects>(self.original(db).0)
            .cloned()
            .unwrap_or_default()
            .0
    }

    pub fn wants_targets(self, db: &mut Database) -> usize {
        let controller = self.original(db).controller(db);
        self.effects(db)
            .into_iter()
            .map(|effect| effect.wants_targets(db, controller))
            .sum::<usize>()
    }

    pub fn source(self, db: &mut Database) -> CardId {
        db.abilities
            .get::<CardId>(self.original(db).0)
            .copied()
            .unwrap()
    }

    pub fn controller(self, db: &mut Database) -> Controller {
        self.source(db).controller(db)
    }

    pub fn delete(self, db: &mut Database) {
        db.abilities.despawn(self.0);
    }

    pub fn short_text(self, db: &mut Database) -> String {
        let mut text = self.text(db);
        if text.len() > 10 {
            text.truncate(10);
            text.push_str("...");
        }

        text
    }

    pub(crate) fn settle(self, db: &mut Database) {
        db.abilities.entity_mut(self.0).insert(Settled);
    }

    pub(crate) fn can_be_activated(
        self,
        db: &mut Database,
        all_players: &AllPlayers,
        turn: &Turn,
    ) -> bool {
        let source = self.source(db);
        let in_battlefield = source.is_in_location::<OnBattlefield>(db);

        match self.ability(db) {
            Ability::Activated(ability) => {
                if !in_battlefield {
                    return false;
                }

                let controller = source.controller(db);
                let is_sorcery = ability
                    .effects
                    .iter()
                    .any(|effect| effect.effect(db, controller).is_sorcery_speed());
                if is_sorcery {
                    if controller != turn.active_player() {
                        return false;
                    }

                    if !matches!(
                        turn.phase,
                        Phase::PreCombatMainPhase | Phase::PostCombatMainPhase
                    ) {
                        return false;
                    }

                    if !Stack::is_empty(db) {
                        return false;
                    }
                }

                can_pay_costs(&ability.cost, source, db, all_players)
            }
            Ability::Mana(ability) => {
                if !in_battlefield {
                    return false;
                };

                can_pay_costs(&ability.cost, source, db, all_players)
            }
            Ability::ETB { .. } => false,
        }
    }
}

fn can_pay_costs(
    cost: &AbilityCost,
    source: CardId,
    db: &mut Database,
    all_players: &AllPlayers,
) -> bool {
    if cost.tap && source.tapped(db) {
        return false;
    }
    let controller = source.controller(db);

    for cost in cost.additional_cost.iter() {
        match cost {
            AdditionalCost::SacrificeThis => {
                if !source.can_be_sacrificed(db) {
                    return false;
                }
            }
            AdditionalCost::PayLife(life) => {
                if all_players[controller].life_total <= life.count as i32 {
                    return false;
                }
            }
        }
    }

    if !all_players[controller].can_spend_mana(&cost.mana_cost) {
        return false;
    }

    true
}
