use crate::{
    action_result::ActionResult,
    effects::{EffectBehaviors, ModifyBattlefield},
    in_play::{Database, ModifierId},
    pending_results::PendingResults,
    player::Controller,
    protogen::{self, effects::Duration, targets::Restriction},
    stack::ActiveTarget,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct BattlefieldModifier {
    pub(crate) modifier: ModifyBattlefield,
    pub(crate) duration: protobuf::EnumOrUnknown<Duration>,
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::BattlefieldModifier> for BattlefieldModifier {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::BattlefieldModifier) -> Result<Self, Self::Error> {
        Ok(Self {
            modifier: value.modifier.get_or_default().try_into()?,
            duration: value.duration,
            restrictions: value.restrictions.clone(),
        })
    }
}

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
