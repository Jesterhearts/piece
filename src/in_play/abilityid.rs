use std::{cell::OnceCell, collections::HashMap, rc::Rc, sync::atomic::Ordering};

use bevy_ecs::{component::Component, entity::Entity};
use derive_more::From;
use itertools::Itertools;

use crate::{
    abilities::{Ability, ActivatedAbility, ApplyToSelf, GainMana, GainManaAbility, SorcerySpeed},
    card::OracleText,
    controller::ControllerRestriction,
    cost::{AbilityCost, AdditionalCost},
    effects::{AnyEffect, Effects},
    in_play::{CardId, Database, InStack, OnBattlefield, NEXT_STACK_SEQ},
    mana::Mana,
    player::{mana_pool::ManaSource, AllPlayers, Controller},
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

                if ability.sorcery_speed {
                    entity.insert(SorcerySpeed);
                }

                Self(entity.id())
            }
            Ability::Mana(ability) => {
                let mut entity = db.abilities.spawn((cardid, ability.cost, ability.gain));
                if let Some(source) = ability.mana_source {
                    entity.insert(source);
                }

                Self(entity.id())
            }
            Ability::ETB { effects } => {
                let entity = db.abilities.spawn((cardid, Effects(effects)));
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

    pub fn move_to_stack(self, db: &mut Database, source: CardId, targets: Vec<Vec<ActiveTarget>>) {
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

    pub fn is_sorcery_speed(self, db: &Database) -> bool {
        db.abilities.get::<SorcerySpeed>(self.0).is_some()
    }

    pub fn ability(self, db: &mut Database) -> Ability {
        let this = self.original(db);

        if let Some((cost, effects, text, apply_to_self, sourcery_speed)) = db
            .abilities
            .query::<(
                Entity,
                &AbilityCost,
                &Effects,
                Option<&OracleText>,
                Option<&ApplyToSelf>,
                Option<&SorcerySpeed>,
            )>()
            .iter(&db.abilities)
            .filter_map(|(e, cost, effect, text, apply_to_self, sorcery_speed)| {
                if Self(e) == this {
                    Some((cost, effect, text, apply_to_self, sorcery_speed))
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
                sorcery_speed: sourcery_speed.is_some(),
            })
        } else if let Some(effects) = db
            .abilities
            .query::<(Entity, &Effects)>()
            .iter(&db.abilities)
            .filter_map(
                |(e, effects)| {
                    if Self(e) == this {
                        Some(effects)
                    } else {
                        None
                    }
                },
            )
            .next()
        {
            Ability::ETB {
                effects: effects.0.clone(),
            }
        } else {
            Ability::Mana(this.gain_mana_ability(db).unwrap())
        }
    }

    pub fn gain_mana_ability(self, db: &mut Database) -> Option<GainManaAbility> {
        db.abilities
            .query::<(Entity, &AbilityCost, &GainMana, Option<&ManaSource>)>()
            .iter(&db.abilities)
            .filter_map(|(e, cost, effect, mana_source)| {
                if Self(e) == self {
                    Some((cost, effect, mana_source))
                } else {
                    None
                }
            })
            .next()
            .map(|(cost, gain, source)| GainManaAbility {
                cost: cost.clone(),
                gain: gain.clone(),
                mana_source: source.copied(),
            })
    }

    pub fn text(self, db: &mut Database) -> String {
        match self.ability(db) {
            Ability::Activated(activated) => {
                format!("{}: {}", activated.cost.text(), activated.oracle_text)
            }
            Ability::Mana(ability) => ability.text(),
            Ability::ETB { effects } => {
                let text = effects
                    .iter()
                    .map(|effect| effect.oracle_text.clone())
                    .join(", ");
                if text.is_empty() {
                    "ETB".to_string()
                } else {
                    text
                }
            }
        }
    }

    pub fn apply_to_self(self, db: &Database) -> bool {
        db.abilities
            .get::<ApplyToSelf>(self.original(db).0)
            .is_some()
    }

    pub fn effects(self, db: &Database) -> Vec<AnyEffect> {
        db.abilities
            .get::<Effects>(self.original(db).0)
            .cloned()
            .unwrap_or_default()
            .0
    }

    pub fn needs_targets(self, db: &mut Database) -> Vec<usize> {
        let controller = self.original(db).controller(db);
        self.effects(db)
            .into_iter()
            .map(|effect| effect.needs_targets(db, controller))
            .collect_vec()
    }

    pub fn wants_targets(self, db: &mut Database) -> Vec<usize> {
        let controller = self.original(db).controller(db);
        self.effects(db)
            .into_iter()
            .map(|effect| effect.effect(db, controller).wants_targets())
            .collect_vec()
    }

    pub fn source(self, db: &Database) -> CardId {
        db.abilities
            .get::<CardId>(self.original(db).0)
            .copied()
            .unwrap()
    }

    pub fn controller(self, db: &Database) -> Controller {
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

    pub fn settle(self, db: &mut Database) {
        db.abilities.entity_mut(self.0).insert(Settled);
    }

    pub fn can_be_activated(
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
                let is_sorcery = ability.sorcery_speed
                    || ability
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

                if !can_pay_costs(db, all_players, &ability.cost, source) {
                    return false;
                }

                let targets = source.targets_for_ability(db, self);
                let needs_targets = self.needs_targets(db);

                needs_targets
                    .into_iter()
                    .zip(targets)
                    .all(|(needs, has)| has.len() >= needs)
            }
            Ability::Mana(ability) => {
                if !in_battlefield {
                    return false;
                };

                can_pay_costs(db, all_players, &ability.cost, source)
            }
            Ability::ETB { .. } => false,
        }
    }

    pub(crate) fn mana_source(self, db: &Database) -> Option<ManaSource> {
        db.abilities.get::<ManaSource>(self.0).copied()
    }
}

fn can_pay_costs(
    db: &mut Database,
    all_players: &AllPlayers,
    cost: &AbilityCost,
    source: CardId,
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
            AdditionalCost::SacrificePermanent(restrictions) => {
                let any_target =
                    controller
                        .get_cards::<OnBattlefield>(db)
                        .into_iter()
                        .any(|card| {
                            card.passes_restrictions(
                                db,
                                source,
                                ControllerRestriction::You,
                                restrictions,
                            )
                        });
                if !any_target {
                    return false;
                }
            }
            AdditionalCost::TapPermanent(restrictions) => {
                let any_target =
                    controller
                        .get_cards::<OnBattlefield>(db)
                        .into_iter()
                        .any(|card| {
                            !card.tapped(db)
                                && card.passes_restrictions(
                                    db,
                                    source,
                                    ControllerRestriction::You,
                                    restrictions,
                                )
                        });

                if !any_target {
                    return false;
                }
            }
        }
    }

    if !all_players[controller].can_meet_cost(&cost.mana_cost, &[]) {
        return false;
    }

    true
}
