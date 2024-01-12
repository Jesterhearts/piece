use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::{Database, ModifierId},
    pending_results::PendingResults,
    player::Controller,
    protogen::effects::BattlefieldModifier,
    stack::ActiveTarget,
};

impl EffectBehaviors for BattlefieldModifier {
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
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: crate::in_play::CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        results.push_settled(ActionResult::AddModifier {
            modifier: ModifierId::upload_temporary_modifier(db, source, self.clone()),
        });
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        _targets: Vec<ActiveTarget>,
        apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        if apply_to_self {
            let modifier = ModifierId::upload_temporary_modifier(db, source, self.clone());
            results.push_settled(ActionResult::ModifyCreatures {
                modifier,
                targets: vec![ActiveTarget::Battlefield { id: source }],
            });
        } else {
            results.push_settled(ActionResult::ApplyToBattlefield(
                ModifierId::upload_temporary_modifier(db, source, self.clone()),
            ));
        }
    }
}
