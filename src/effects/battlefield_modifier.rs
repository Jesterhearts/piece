use anyhow::anyhow;

use crate::{
    battlefield::{ActionResult, PendingResults},
    controller::ControllerRestriction,
    effects::{EffectBehaviors, EffectDuration, ModifyBattlefield},
    in_play::{CardId, Database, ModifierId},
    player::Controller,
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct BattlefieldModifier {
    pub(crate) modifier: ModifyBattlefield,
    pub(crate) controller: ControllerRestriction,
    pub(crate) duration: EffectDuration,
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::BattlefieldModifier> for BattlefieldModifier {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::BattlefieldModifier) -> Result<Self, Self::Error> {
        Ok(Self {
            modifier: value.modifier.get_or_default().try_into()?,
            controller: value
                .controller
                .controller
                .as_ref()
                .ok_or_else(|| anyhow!("Expected battlefield modifier to have a controller set"))?
                .try_into()?,
            duration: value
                .duration
                .duration
                .as_ref()
                .ok_or_else(|| anyhow!("Expected duration to have a duration specified"))
                .map(EffectDuration::from)?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

impl EffectBehaviors for BattlefieldModifier {
    fn needs_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        0
    }

    fn wants_targets(&'static self, _db: &mut crate::in_play::Database) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        results.push_settled(ActionResult::AddModifier {
            modifier: ModifierId::upload_temporary_modifier(db, source, self),
        });
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        _targets: Vec<ActiveTarget>,
        apply_to_self: bool,
        source: CardId,
        _controller: Controller,
        results: &mut PendingResults,
    ) {
        if apply_to_self {
            let modifier = ModifierId::upload_temporary_modifier(db, source, self);
            results.push_settled(ActionResult::ModifyCreatures {
                modifier,
                targets: vec![ActiveTarget::Battlefield { id: source }],
            });
        } else {
            results.push_settled(ActionResult::ApplyToBattlefield(
                ModifierId::upload_temporary_modifier(db, source, self),
            ));
        }
    }
}
