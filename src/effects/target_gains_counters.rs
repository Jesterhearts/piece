use anyhow::anyhow;
use itertools::Itertools;
use tracing::Level;

use crate::{
    battlefield::ActionResult,
    counters::Counter,
    effects::{Effect, EffectBehaviors},
    in_play::target_from_location,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GainCount {
    Single,
    Multiple(usize),
    Dynamic(DynamicCounter),
}

impl TryFrom<&protogen::effects::gain_counter::Count> for GainCount {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::gain_counter::Count) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::gain_counter::Count::Single(_) => Ok(Self::Single),
            protogen::effects::gain_counter::Count::Multiple(value) => {
                Ok(Self::Multiple(usize::try_from(value.count)?))
            }
            protogen::effects::gain_counter::Count::Dynamic(dynamic) => {
                Ok(Self::Dynamic(dynamic.try_into()?))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            counter: (&value.counter).try_into()?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for TargetGainsCounters {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in db.cards.keys() {
            if card.passes_restrictions(db, log_session, source, &self.restrictions)
                && card.passes_restrictions(
                    db,
                    log_session,
                    source,
                    &source.faceup_face(db).restrictions,
                )
                && card.can_be_targeted(db, controller)
            {
                let target = target_from_location(db, *card);
                if already_chosen.contains(&target) {
                    continue;
                }
                targets.push(target);
            }
        }

        targets
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let valid_targets = self.valid_targets(
            db,
            source,
            crate::log::LogId::current(db),
            controller,
            results.all_currently_targeted(),
        );

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            crate::log::LogId::current(db),
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
        results: &mut crate::pending_results::PendingResults,
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
