use std::collections::{HashMap, HashSet};

use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::{
    effects::{ApplyResult, EffectBundle, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    player::{Controller, Owner},
    protogen::{
        color::Color,
        effects::{
            dest::Destination,
            pay_cost::PayMana,
            static_ability::{self},
            Dest, Duration, Effect, MoveToBattlefield, MoveToGraveyard, PayCost, PayCosts,
            PushSelected, SelectDestinations,
        },
        mana::{spend_reason::Activating, SpendReason},
        targets::Location,
        types::Type,
    },
    stack::{Selected, TargetType},
    types::TypeSet,
};

#[derive(Debug, Default)]
pub struct Battlefields {
    pub battlefields: IndexMap<Controller, IndexSet<CardId>>,
}

impl std::ops::Index<Owner> for Battlefields {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Owner) -> &Self::Output {
        self.battlefields.get(&Controller::from(index)).unwrap()
    }
}

impl std::ops::Index<Controller> for Battlefields {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Controller) -> &Self::Output {
        self.battlefields.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<Owner> for Battlefields {
    fn index_mut(&mut self, index: Owner) -> &mut Self::Output {
        self.battlefields
            .entry(Controller::from(index))
            .or_default()
    }
}

impl std::ops::IndexMut<Controller> for Battlefields {
    fn index_mut(&mut self, index: Controller) -> &mut Self::Output {
        self.battlefields.entry(index).or_default()
    }
}

impl Battlefields {
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.battlefields.values().all(|cards| cards.is_empty())
    }

    #[cfg(test)]
    pub(crate) fn no_modifiers(db: &Database) -> bool {
        db.modifiers.values().all(|modifier| !modifier.active)
    }

    pub(crate) fn controlled_colors(db: &Database, player: Controller) -> HashSet<Color> {
        let mut colors = HashSet::default();
        for card in db.battlefield[player].as_slice() {
            colors.extend(db[*card].modified_colors.iter().copied())
        }

        colors
    }

    pub(crate) fn untap(db: &mut Database, player: Owner) {
        let cards = db
            .battlefield
            .battlefields
            .iter()
            .flat_map(|(controller, cards)| cards.iter().map(|card| (*controller, *card)))
            .filter_map(|(controller, card)| {
                if controller == player
                    || db[card].modified_static_abilities.iter().any(|ability| {
                        matches!(
                            db[*ability].ability,
                            static_ability::Ability::UntapEachUntapStep(_)
                        )
                    })
                {
                    Some(card)
                } else {
                    None
                }
            })
            .collect_vec();

        for card in cards {
            card.untap(db);
        }
    }

    pub(crate) fn end_turn(db: &mut Database) -> PendingEffects {
        for card in db.battlefield.battlefields.values().flat_map(|b| b.iter()) {
            db.cards.entry(*card).or_default().marked_damage = 0;
        }

        let mut results = PendingEffects::default();
        let returning = db
            .exile
            .exile_zones
            .values()
            .flat_map(|e| e.iter())
            .copied()
            .filter(|card| db[*card].exile_duration == Some(Duration::UNTIL_END_OF_TURN))
            .map(|card| Selected {
                location: Some(Location::IN_EXILE),
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: vec![],
            })
            .collect_vec();

        results.push_back(EffectBundle {
            selected: SelectedStack::new(returning),
            effects: vec![Effect {
                effect: Some(MoveToBattlefield::default().into()),
                ..Default::default()
            }],
            ..Default::default()
        });

        let all_modifiers = db
            .modifiers
            .iter()
            .filter_map(|(id, modifier)| {
                if modifier.active
                    && matches!(
                        modifier.modifier.duration.enum_value().unwrap(),
                        Duration::UNTIL_END_OF_TURN
                    )
                {
                    Some(id)
                } else {
                    None
                }
            })
            .copied()
            .collect_vec();

        for modifier in all_modifiers {
            modifier.deactivate(db);
        }

        for card in db.cards.keys().copied().collect_vec() {
            card.apply_modifiers_layered(db);
        }

        results
    }

    pub fn check_sba(db: &mut Database) -> PendingEffects {
        let mut pending = PendingEffects::default();

        let mut legendary_cards: HashMap<String, Vec<CardId>> = HashMap::default();
        let mut bundle = EffectBundle {
            effects: vec![Effect {
                effect: Some(MoveToGraveyard::default().into()),
                ..Default::default()
            }],
            ..Default::default()
        };

        for card in db
            .battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
        {
            if card.types_intersect(db, &TypeSet::from([Type::LEGENDARY])) {
                legendary_cards
                    .entry(db[card].modified_name.clone())
                    .or_default()
                    .push(card);
            }

            let toughness = card.toughness(db);

            if toughness.is_some()
                && (toughness.unwrap() <= 0
                    || ((toughness.unwrap() - card.marked_damage(db)) <= 0
                        && !card.indestructible(db)))
            {
                bundle.selected.push(Selected {
                    location: Some(Location::ON_BATTLEFIELD),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                });
            }

            let enchanting = db[card].enchanting;
            if enchanting.is_some()
                && !enchanting
                    .unwrap()
                    .is_in_location(db, Location::ON_BATTLEFIELD)
            {
                bundle.selected.push(Selected {
                    location: Some(Location::ON_BATTLEFIELD),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                });
            }
        }

        pending.push_back(bundle);

        for legends in legendary_cards.values() {
            if legends.len() > 1 {
                pending.push_back(EffectBundle {
                    selected: SelectedStack::new(
                        legends
                            .iter()
                            .copied()
                            .map(|legend| Selected {
                                location: Some(Location::ON_BATTLEFIELD),
                                target_type: TargetType::Card(legend),
                                targeted: false,
                                restrictions: vec![],
                            })
                            .collect_vec(),
                    ),
                    effects: vec![Effect {
                        effect: Some(
                            SelectDestinations {
                                destinations: vec![Dest {
                                    count: (legends.len() - 1) as u32,
                                    destination: Some(
                                        Destination::from(MoveToGraveyard::default()),
                                    ),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            }
                            .into(),
                        ),
                        ..Default::default()
                    }],
                    ..Default::default()
                });
            }
        }

        pending
    }

    pub fn activate_ability(
        db: &mut Database,
        pending: &Option<PendingEffects>,
        activator: Owner,
        source: CardId,
        index: usize,
    ) -> PendingEffects {
        if db.stack.split_second(db) {
            debug!("Can't activate ability (split second)");
            return PendingEffects::default();
        }

        let (_, ability) = db[source].abilities(db).into_iter().nth(index).unwrap();

        if !ability.can_be_activated(db, source, activator, pending) {
            debug!("Can't activate ability (can't meet costs)");
            return PendingEffects::default();
        }

        let mut results = PendingEffects::default();
        let mut bundle = EffectBundle {
            source: Some(source),
            ..Default::default()
        };
        if let Some(targets) = ability.targets(db) {
            bundle.effects.push(targets.clone().into());

            if ability.is_craft(db) {
                bundle.selected.crafting = true
            }

            bundle.effects.push(PushSelected::default().into());
        }

        if let Some(additional_costs) = ability.additional_costs(db) {
            bundle.effects.extend(additional_costs.iter().cloned());
        }

        if let Some(cost) = ability.cost(db) {
            bundle.effects.push(Effect {
                effect: Some(
                    PayCosts {
                        pay_costs: vec![PayCost {
                            cost: Some(
                                PayMana {
                                    paying: cost.mana_cost.iter().cloned().sorted().collect_vec(),
                                    reason: protobuf::MessageField::some(SpendReason {
                                        reason: Some(
                                            Activating {
                                                source: protobuf::MessageField::some(source.into()),
                                                ..Default::default()
                                            }
                                            .into(),
                                        ),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }
                                .into(),
                            ),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            });
        }

        results.push_back(bundle);

        let mut bundle = EffectBundle {
            source: Some(source),
            effects: ability.effects(db),
            ..Default::default()
        };
        if ability.is_craft(db) {
            bundle.selected.crafting = true
        }
        results.push_back(bundle);

        results
    }

    pub(crate) fn static_abilities(db: &Database) -> Vec<(&static_ability::Ability, CardId)> {
        let mut result: Vec<(&static_ability::Ability, CardId)> = Default::default();

        for card in db
            .battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
        {
            for ability in db[card].modified_static_abilities.iter() {
                result.push((&db[*ability].ability, card));
            }
        }

        result
    }

    pub(crate) fn maybe_leave_battlefield(
        db: &mut Database,
        target: CardId,
    ) -> Option<ApplyResult> {
        if !db.battlefield[db[target].controller].contains(&target) {
            return None;
        }

        for modifier in db
            .modifiers
            .iter()
            .filter_map(|(id, modifier)| {
                if (matches!(
                    modifier.modifier.duration.enum_value().unwrap(),
                    Duration::UNTIL_SOURCE_LEAVES_BATTLEFIELD
                ) && modifier.source == target)
                    || (matches!(
                        modifier.modifier.duration.enum_value().unwrap(),
                        Duration::UNTIL_TARGET_LEAVES_BATTLEFIELD
                    ) && modifier.modifying.contains(&target))
                {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect_vec()
        {
            modifier.deactivate(db);
        }

        db[target].left_battlefield_turn = Some(db.turn.turn_count);

        let selected = db[target]
            .exiling
            .iter()
            .copied()
            .filter(|card| {
                matches!(
                    db[*card].exile_duration,
                    Some(Duration::UNTIL_SOURCE_LEAVES_BATTLEFIELD)
                )
            })
            .map(|card| Selected {
                location: Some(Location::IN_EXILE),
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: vec![],
            })
            .collect_vec();

        Some(ApplyResult::PushBack(EffectBundle {
            selected: SelectedStack::new(selected),
            effects: vec![Effect {
                effect: Some(MoveToBattlefield::default().into()),
                ..Default::default()
            }],
            ..Default::default()
        }))
    }
}
