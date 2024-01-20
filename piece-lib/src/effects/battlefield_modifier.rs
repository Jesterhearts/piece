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
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: &crate::protogen::ids::CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        results.push_settled(ActionResult::AddModifier {
            modifier: ModifierId::upload_temporary_modifier(db, source.clone(), self.clone()),
        });
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        _targets: Vec<ActiveTarget>,
        source: &crate::protogen::ids::CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        if self.apply_to_self {
            let modifier = ModifierId::upload_temporary_modifier(db, source.clone(), self.clone());
            results.push_settled(ActionResult::ModifyCreatures {
                modifier,
                targets: vec![ActiveTarget::Battlefield { id: source.clone() }],
            });
        } else {
            results.push_settled(ActionResult::ApplyToBattlefield(
                ModifierId::upload_temporary_modifier(db, source.clone(), self.clone()),
            ));
        }
    }
}
