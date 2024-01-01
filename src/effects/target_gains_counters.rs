use anyhow::anyhow;
use itertools::Itertools;
use tracing::Level;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    effects::{Effect, EffectBehaviors},
    in_play::{self, target_from_location},
    newtype_enum::newtype_enum,
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
#[derive(strum::EnumIter, strum::AsRefStr, Hash)]
pub enum Counter {
    Any,
    Charge,
    P1P1,
    M1M1,
}
}

#[derive(Debug, Clone)]
pub(crate) enum DynamicCounter {
    X,
    LeftBattlefieldThisTurn { restrictions: Vec<Restriction> },
}

impl TryFrom<&protogen::effects::gain_counter::Dynamic> for DynamicCounter {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::gain_counter::Dynamic) -> Result<Self, Self::Error> {
        value
            .dynamic
            .as_ref()
            .ok_or_else(|| anyhow!("Expected dynamic counter to have a value set"))
            .and_then(Self::try_from)
    }
}

impl TryFrom<&protogen::effects::gain_counter::dynamic::Dynamic> for DynamicCounter {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::gain_counter::dynamic::Dynamic,
    ) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::gain_counter::dynamic::Dynamic::LeftBattlefieldThisTurn(value) => {
                Ok(Self::LeftBattlefieldThisTurn {
                    restrictions: value
                        .restrictions
                        .iter()
                        .map(Restriction::try_from)
                        .collect::<anyhow::Result<_>>()?,
                })
            }
            protogen::effects::gain_counter::dynamic::Dynamic::X(_) => Ok(Self::X),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum GainCount {
    Single,
    Dynamic(DynamicCounter),
}

impl TryFrom<&protogen::effects::gain_counter::Count> for GainCount {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::gain_counter::Count) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::gain_counter::Count::Single(_) => Ok(Self::Single),
            protogen::effects::gain_counter::Count::Dynamic(dynamic) => {
                Ok(Self::Dynamic(dynamic.try_into()?))
            }
        }
    }
}

impl TryFrom<&protogen::counters::Counter> for Counter {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::counters::Counter) -> Result<Self, Self::Error> {
        value
            .type_
            .as_ref()
            .ok_or_else(|| anyhow!("Expected counter to have a type specified"))
            .map(Self::from)
    }
}

impl From<&protogen::counters::counter::Type> for Counter {
    fn from(value: &protogen::counters::counter::Type) -> Self {
        match value {
            protogen::counters::counter::Type::Any(_) => Self::Any,
            protogen::counters::counter::Type::Charge(_) => Self::Charge,
            protogen::counters::counter::Type::P1p1(_) => Self::P1P1,
            protogen::counters::counter::Type::M1m1(_) => Self::M1M1,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TargetGainsCounters {
    count: GainCount,
    counter: Counter,
    restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::GainCounter> for TargetGainsCounters {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::GainCounter) -> Result<Self, Self::Error> {
        Ok(Self {
            count: value
                .count
                .as_ref()
                .ok_or_else(|| anyhow!("Expected counter to have a counter specified"))
                .and_then(GainCount::try_from)?,
            counter: value.counter.get_or_default().try_into()?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for TargetGainsCounters {
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in in_play::all_cards(db) {
            if card.passes_restrictions(db, source, &self.restrictions)
                && card.passes_restrictions(db, source, &source.restrictions(db))
                && card.can_be_targeted(db, controller)
            {
                let target = target_from_location(db, card);
                if already_chosen.contains(&target) {
                    continue;
                }
                targets.push(target);
            }
        }

        targets
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect(self)),
            valid_targets,
            source,
        ));
    }

    #[instrument(level = Level::INFO, skip(_db, results))]
    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if let Ok(target) = targets.into_iter().exactly_one() {
            let target = match target {
                ActiveTarget::Battlefield { id } => id,
                ActiveTarget::Graveyard { id } => id,
                _ => unreachable!(),
            };

            results.push_settled(ActionResult::AddCounters {
                source,
                target,
                count: self.count.clone(),
                counter: self.counter,
            });
        } else {
            warn!("Skpping targets");
        }
    }
}
