use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    effects::{BattlefieldModifier, Effect, EffectBehaviors, EffectDuration},
    in_play::{target_from_location, Database, ModifierId},
    pending_results::{choose_targets::ChooseTargets, PendingResults, TargetSource},
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
        db: &Database,
        source: crate::in_play::CardId,
        controller: Controller,
        already_chosen: &std::collections::HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        let mut targets = vec![];
        for card in db.cards.keys() {
            if card.can_be_targeted(db, controller)
                && card.passes_restrictions(db, source, &source.faceup_face(db).restrictions)
                && card.passes_restrictions(db, source, &self.restrictions)
            {
                let target = target_from_location(db, *card);
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
            }
        }

        targets
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: crate::in_play::CardId,
        controller: Controller,
        results: &mut PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());
        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            source,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
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
                BattlefieldModifier {
                    modifier: self.modifier.clone(),
                    duration: self.duration,
                    restrictions: vec![],
                },
            ),
            _ => ModifierId::upload_temporary_modifier(
                db,
                source,
                BattlefieldModifier {
                    modifier: self.modifier.clone(),
                    duration: self.duration,
                    restrictions: vec![],
                },
            ),
        };

        results.push_settled(ActionResult::ModifyCreatures {
            targets: final_targets,
            modifier,
        });
    }
}
