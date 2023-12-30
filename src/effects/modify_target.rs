use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, PendingResults, TargetSource},
    controller::ControllerRestriction,
    effects::{BattlefieldModifier, Effect, EffectBehaviors, EffectDuration},
    in_play::{self, target_from_location, CardId, Database, ModifierId},
    player::Controller,
    protogen,
    stack::ActiveTarget,
};

#[derive(Debug, Clone, Deref, DerefMut)]
pub(crate) struct ModifyTarget(pub(crate) BattlefieldModifier);

impl TryFrom<&protogen::effects::BattlefieldModifier> for ModifyTarget {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::BattlefieldModifier) -> Result<Self, Self::Error> {
        Ok(Self(BattlefieldModifier::try_from(value)?))
    }
}

impl EffectBehaviors for ModifyTarget {
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        already_chosen: &std::collections::HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        let mut targets = vec![];
        for card in in_play::all_cards(db) {
            if card.can_be_targeted(db, controller)
                && card.passes_restrictions(
                    db,
                    source,
                    ControllerRestriction::Any,
                    &source.restrictions(db),
                )
                && card.passes_restrictions(db, source, self.controller, &self.restrictions)
            {
                let target = target_from_location(db, card);
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
            }
        }

        targets
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());
        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect(self)),
            valid_targets,
            source,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        _apply_to_self: bool,
        source: CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        let mut final_targets = vec![];
        for target in targets {
            match target {
                ActiveTarget::Battlefield { .. } => {
                    final_targets.push(target);
                }
                ActiveTarget::Graveyard { .. } => {
                    final_targets.push(target);
                }
                _ => unreachable!(),
            }
        }

        let modifier = match self.duration {
            EffectDuration::UntilTargetLeavesBattlefield => ModifierId::upload_temporary_modifier(
                db,
                final_targets.iter().exactly_one().unwrap().id().unwrap(),
                self,
            ),
            _ => ModifierId::upload_temporary_modifier(db, source, self),
        };

        results.push_settled(ActionResult::ModifyCreatures {
            targets: final_targets,
            modifier,
        });
    }
}
