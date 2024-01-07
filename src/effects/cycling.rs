use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors},
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
    types::{Subtype, Type},
};

#[derive(Debug, Clone)]
pub(crate) struct Cycling {
    types: IndexSet<Type>,
    subtypes: IndexSet<Subtype>,
}

impl TryFrom<&protogen::effects::Cycling> for Cycling {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Cycling) -> Result<Self, Self::Error> {
        Ok(Self {
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<_>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for Cycling {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        if !self.types.is_empty() || !self.subtypes.is_empty() {
            1
        } else {
            0
        }
    }

    fn cycling(&self) -> bool {
        true
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        _already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        if self.types.is_empty() && self.subtypes.is_empty() {
            return vec![];
        }

        let restrictions = [Restriction::OfType {
            types: self.types.clone(),
            subtypes: self.subtypes.clone(),
        }];

        db.all_players[controller]
            .library
            .cards
            .iter()
            .filter(|card| card.passes_restrictions(db, source, &restrictions))
            .map(|card| ActiveTarget::Library { id: *card })
            .collect_vec()
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if self.types.is_empty() && self.subtypes.is_empty() {
            results.push_settled(ActionResult::DrawCards {
                target: controller,
                count: 1,
            })
        } else {
            let valid_targets =
                self.valid_targets(db, source, controller, results.all_currently_targeted());
            results.push_choose_targets(ChooseTargets::new(
                TargetSource::Effect(Effect::from(self.clone())),
                valid_targets,
                source,
            ));
        }
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if self.types.is_empty() && self.subtypes.is_empty() {
            results.push_settled(ActionResult::DrawCards {
                target: controller,
                count: 1,
            });
        } else {
            for target in targets {
                let ActiveTarget::Library { id } = target else {
                    unreachable!()
                };

                results.push_settled(ActionResult::RevealCard(id));
                results.push_settled(ActionResult::MoveToHandFromLibrary(id));
            }
        }
    }
}
