use bevy_ecs::{component::Component, entity::Entity, query::With};
use derive_more::{Deref, DerefMut};

use indexmap::IndexMap;
use strum::IntoEnumIterator;

use crate::{
    counters::{counter, Counter},
    in_play::{CardId, Database},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref, DerefMut, Component)]
pub(crate) struct Count(pub(crate) usize);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub(crate) struct CounterId(Entity);

impl CounterId {
    pub(crate) fn add_counters(db: &mut Database, card: CardId, counter: Counter, count: usize) {
        match counter {
            Counter::Any => {
                for counter in Counter::iter() {
                    if let Counter::Any = counter {
                        continue;
                    }
                    Self::add_counters(db, card, counter, count);
                }
            }
            Counter::Charge => Self::add_counters_of_type::<counter::Charge>(db, card, count),
            Counter::Net => Self::add_counters_of_type::<counter::Net>(db, card, count),
            Counter::P1P1 => Self::add_counters_of_type::<counter::P1P1>(db, card, count),
            Counter::M1M1 => Self::add_counters_of_type::<counter::M1M1>(db, card, count),
        }
    }

    pub(crate) fn remove_counters(db: &mut Database, card: CardId, counter: Counter, count: usize) {
        match counter {
            Counter::Any => {
                for counter in Counter::iter() {
                    if let Counter::Any = counter {
                        continue;
                    }
                    Self::remove_counters(db, card, counter, count);
                }
            }
            Counter::Charge => Self::remove_counters_of_type::<counter::Charge>(db, card, count),
            Counter::Net => Self::remove_counters_of_type::<counter::Net>(db, card, count),
            Counter::P1P1 => Self::remove_counters_of_type::<counter::P1P1>(db, card, count),
            Counter::M1M1 => Self::remove_counters_of_type::<counter::M1M1>(db, card, count),
        }
    }

    pub(crate) fn add_counters_of_type<Type: Component + Default>(
        db: &mut Database,
        card: CardId,
        count: usize,
    ) {
        let existing = db
            .counters
            .query_filtered::<(&CardId, &mut Count), With<Type>>()
            .iter_mut(&mut db.counters)
            .find_map(
                |(is_on, count)| {
                    if card == *is_on {
                        Some(count)
                    } else {
                        None
                    }
                },
            );

        if let Some(mut existing_count) = existing {
            **existing_count += count;
        } else {
            db.counters.spawn((card, Count(count), Type::default()));
        }
    }

    pub(crate) fn remove_counters_of_type<Type: Component + Default>(
        db: &mut Database,
        card: CardId,
        count: usize,
    ) {
        let existing = db
            .counters
            .query_filtered::<(&CardId, &mut Count), With<Type>>()
            .iter_mut(&mut db.counters)
            .find_map(
                |(is_on, count)| {
                    if card == *is_on {
                        Some(count)
                    } else {
                        None
                    }
                },
            );

        if let Some(mut existing_count) = existing {
            **existing_count = existing_count.saturating_sub(count);
        } else {
            db.counters.spawn((card, Count(count), Type::default()));
        }
    }

    pub(crate) fn counters_on(db: &mut Database, card: CardId, counter: Counter) -> usize {
        match counter {
            Counter::Any => {
                let mut sum = 0;
                for counter in Counter::iter() {
                    if let Counter::Any = counter {
                        continue;
                    }
                    sum += Self::counters_on(db, card, counter);
                }
                sum
            }
            Counter::Charge => Self::counters_of_type_on::<counter::Charge>(db, card),
            Counter::Net => Self::counters_of_type_on::<counter::Net>(db, card),
            Counter::P1P1 => Self::counters_of_type_on::<counter::P1P1>(db, card),
            Counter::M1M1 => Self::counters_of_type_on::<counter::M1M1>(db, card),
        }
    }

    pub(crate) fn counters_of_type_on<Type: Component>(db: &mut Database, card: CardId) -> usize {
        db.counters
            .query_filtered::<(&CardId, &Count), With<Type>>()
            .iter_mut(&mut db.counters)
            .find_map(
                |(is_on, count)| {
                    if card == *is_on {
                        Some(**count)
                    } else {
                        None
                    }
                },
            )
            .unwrap_or_default()
    }

    pub(crate) fn counter_text_on(db: &mut Database, card: CardId) -> Vec<String> {
        let mut results = vec![];

        for counter in Counter::iter() {
            let amount = Self::counters_on(db, card, counter);
            if amount > 0 {
                results.push(match counter {
                    Counter::Charge => format!("Charge x{}", amount),
                    Counter::Net => format!("Net x{}", amount),
                    Counter::P1P1 => format!("+1/+1 x{}", amount),
                    Counter::M1M1 => format!("-1/-1 x{}", amount),
                    Counter::Any => format!("{} total counters", amount),
                });
            }
        }

        results
    }

    pub(crate) fn all_counters_on(db: &mut Database, card: CardId) -> IndexMap<Counter, usize> {
        let mut results = IndexMap::default();

        for counter in Counter::iter() {
            if let Counter::Any = counter {
                continue;
            }

            let amount = Self::counters_on(db, card, counter);
            results.insert(counter, amount);
        }
        results
    }
}
