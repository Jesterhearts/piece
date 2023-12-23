use anyhow::anyhow;

use crate::{
    battlefield::ActionResult,
    effects::EffectBehaviors,
    newtype_enum::newtype_enum,
    protogen::{self},
};

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
#[derive(strum::EnumIter)]
pub enum Counter {
    Charge,
    P1P1,
    M1M1,
}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicCounter {
    X(Counter),
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
            protogen::effects::gain_counter::dynamic::Dynamic::X(counter) => {
                Ok(Self::X(counter.counter.get_or_default().try_into()?))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GainCounter {
    Single(Counter),
    Dynamic(DynamicCounter),
}

impl TryFrom<&protogen::effects::GainCounter> for GainCounter {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::GainCounter) -> Result<Self, Self::Error> {
        value
            .counter
            .as_ref()
            .ok_or_else(|| anyhow!("Expected counter to have a counter specified"))
            .and_then(GainCounter::try_from)
    }
}

impl TryFrom<&protogen::effects::gain_counter::Counter> for GainCounter {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::gain_counter::Counter) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::gain_counter::Counter::Single(counter) => {
                Ok(Self::Single(counter.counter.get_or_default().try_into()?))
            }
            protogen::effects::gain_counter::Counter::Dynamic(dynamic) => {
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
            protogen::counters::counter::Type::Charge(_) => Self::Charge,
            protogen::counters::counter::Type::P1p1(_) => Self::P1P1,
            protogen::counters::counter::Type::M1m1(_) => Self::M1M1,
        }
    }
}

impl EffectBehaviors for GainCounter {
    fn needs_targets(&self) -> usize {
        0
    }

    fn wants_targets(&self) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::AddCounters {
            source,
            target: source,
            counter: *self,
        });
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::AddCounters {
            source,
            target: source,
            counter: *self,
        });
    }
}
