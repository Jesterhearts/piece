use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors},
    protogen::{self, mana::ManaSource},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForEachManaOfSource {
    pub(crate) source: protobuf::EnumOrUnknown<ManaSource>,
    pub(crate) effect: Box<Effect>,
}

impl TryFrom<&protogen::effects::ForEachManaOfSource> for ForEachManaOfSource {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ForEachManaOfSource) -> Result<Self, Self::Error> {
        Ok(Self {
            source: value.source,
            effect: Box::new(value.effect.get_or_default().try_into()?),
        })
    }
}

impl EffectBehaviors for ForEachManaOfSource {
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
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::ForEachManaOfSource {
            card: source,
            source: self.source,
            effect: *self.effect.clone(),
        });
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::ForEachManaOfSource {
            card: source,
            source: self.source,
            effect: *self.effect.clone(),
        });
    }
}
