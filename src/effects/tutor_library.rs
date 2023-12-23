

use itertools::Itertools;

use crate::{
    battlefield::{compute_deck_targets, ActionResult, ChooseTargets, TargetSource},
    effects::{Destination, Effect, EffectBehaviors},
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TutorLibrary {
    pub restrictions: Vec<Restriction>,
    pub destination: Destination,
    pub reveal: bool,
}

impl TryFrom<&protogen::effects::TutorLibrary> for TutorLibrary {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::TutorLibrary) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            destination: value.destination.get_or_default().try_into()?,
            reveal: value.reveal,
        })
    }
}

impl EffectBehaviors for TutorLibrary {
    fn needs_targets(&self) -> usize {
        1
    }

    fn wants_targets(&self) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        _already_chosen: &std::collections::HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        compute_deck_targets(db, controller, &self.restrictions)
            .into_iter()
            .map(|card| ActiveTarget::Library { id: card })
            .collect_vec()
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
        ));
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if self.reveal {
            for target in targets.iter() {
                let ActiveTarget::Library { id } = target else {
                    unreachable!()
                };

                results.push_settled(ActionResult::RevealCard(*id))
            }
        }

        match self.destination {
            Destination::Hand => {
                for target in targets {
                    let ActiveTarget::Library { id } = target else {
                        unreachable!()
                    };
                    results.push_settled(ActionResult::MoveToHandFromLibrary(id))
                }
            }
            Destination::TopOfLibrary => {
                for target in targets {
                    let ActiveTarget::Library { id } = target else {
                        unreachable!()
                    };
                    results.push_settled(ActionResult::MoveFromLibraryToTopOfLibrary(id))
                }
            }
            Destination::Battlefield { enters_tapped } => {
                for target in targets {
                    let ActiveTarget::Library { id } = target else {
                        unreachable!()
                    };
                    results.push_settled(ActionResult::AddToBattlefieldFromLibrary {
                        card: id,
                        enters_tapped,
                    });
                }
            }
        }
    }
}
