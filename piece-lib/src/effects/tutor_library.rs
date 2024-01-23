use itertools::Itertools;

use crate::{
    action_result::{
        add_to_battlefield_from_library::AddToBattlefieldFromLibrary,
        move_from_library_to_graveyard::MoveFromLibraryToGraveyard,
        move_from_library_to_top_of_library::MoveFromLibraryToTopOfLibrary,
        move_to_hand_from_library::MoveToHandFromLibrary, reveal_card::RevealCard,
        shuffle::Shuffle, ActionResult,
    },
    effects::EffectBehaviors,
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::effects::{
        destination::{self, Battlefield},
        effect::Effect,
        TutorLibrary,
    },
    stack::ActiveTarget,
};

impl EffectBehaviors for TutorLibrary {
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
        _already_chosen: &std::collections::HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        db.all_players[controller]
            .library
            .cards
            .iter()
            .filter(|card| {
                card.passes_restrictions(
                    db,
                    log_session,
                    source,
                    &source.faceup_face(db).restrictions,
                ) && card.passes_restrictions(db, log_session, source, &self.restrictions)
            })
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

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        if self.reveal {
            for target in targets.iter() {
                let ActiveTarget::Library { id } = target else {
                    unreachable!()
                };

                results.push_settled(ActionResult::from(RevealCard { card: *id }))
            }
        }

        match self.destination.destination.as_ref().unwrap() {
            destination::Destination::Hand(_) => {
                for target in targets {
                    let ActiveTarget::Library { id } = target else {
                        unreachable!()
                    };
                    results.push_settled(ActionResult::from(MoveToHandFromLibrary { card: id }))
                }
            }
            destination::Destination::TopOfLibrary(_) => {
                for target in targets {
                    let ActiveTarget::Library { id } = target else {
                        unreachable!()
                    };
                    results.push_settled(ActionResult::from(MoveFromLibraryToTopOfLibrary {
                        card: id,
                    }))
                }
            }
            destination::Destination::Battlefield(Battlefield { enters_tapped, .. }) => {
                for target in targets {
                    let ActiveTarget::Library { id } = target else {
                        unreachable!()
                    };
                    results.push_settled(ActionResult::from(AddToBattlefieldFromLibrary {
                        card: id,
                        enters_tapped: *enters_tapped,
                    }));
                }
            }
            destination::Destination::BottomOfLibrary(_) => unreachable!(),
            destination::Destination::Graveyard(_) => {
                for target in targets {
                    let ActiveTarget::Library { id } = target else {
                        unreachable!()
                    };
                    results
                        .push_settled(ActionResult::from(MoveFromLibraryToGraveyard { card: id }));
                }
            }
        }

        results.push_settled(ActionResult::from(Shuffle {
            player: controller.into(),
        }));
    }
}
