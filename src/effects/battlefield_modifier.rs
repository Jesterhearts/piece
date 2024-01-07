use anyhow::anyhow;

use crate::{
    battlefield::ActionResult,
    effects::{EffectBehaviors, EffectDuration, ModifyBattlefield},
    in_play::{Database, ModifierId},
    pending_results::PendingResults,
    player::Controller,
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct BattlefieldModifier {
    pub(crate) modifier: ModifyBattlefield,
    pub(crate) duration: EffectDuration,
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::BattlefieldModifier> for BattlefieldModifier {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::BattlefieldModifier) -> Result<Self, Self::Error> {
        Ok(Self {
            modifier: value.modifier.get_or_default().try_into()?,
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
            modifier: ModifierId::upload_temporary_modifier(
                &mut db.modifiers,
                source,
                self.clone(),
            ),
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
            let modifier =
                ModifierId::upload_temporary_modifier(&mut db.modifiers, source, self.clone());
            results.push_settled(ActionResult::ModifyCreatures {
                modifier,
                targets: vec![ActiveTarget::Battlefield { id: source }],
            });
        } else {
            results.push_settled(ActionResult::ApplyToBattlefield(
                ModifierId::upload_temporary_modifier(&mut db.modifiers, source, self.clone()),
            ));
        }
    }
}
