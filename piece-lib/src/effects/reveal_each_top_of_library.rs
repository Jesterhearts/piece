use crate::{
    action_result::ActionResult,
    effects::{Effect, EffectBehaviors},
    protogen::{self, targets::Restriction},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForEach {
    pub(crate) restrictions: Vec<Restriction>,
    pub(crate) effects: Vec<Effect>,
    pub(crate) if_none: Vec<Effect>,
}

impl TryFrom<&protogen::effects::reveal_each_top_of_library::ForEach> for ForEach {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::reveal_each_top_of_library::ForEach,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value.restrictions.clone(),
            effects: value
                .effects
                .iter()
                .map(Effect::try_from)
                .collect::<anyhow::Result<_>>()?,
            if_none: value
                .if_none
                .effects
                .iter()
                .map(Effect::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevealEachTopOfLibrary {
    pub(crate) for_each: ForEach,
}

impl TryFrom<&protogen::effects::RevealEachTopOfLibrary> for RevealEachTopOfLibrary {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::RevealEachTopOfLibrary) -> Result<Self, Self::Error> {
        Ok(Self {
            for_each: value.for_each.get_or_default().try_into()?,
        })
    }
}

impl EffectBehaviors for RevealEachTopOfLibrary {
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
        results.push_settled(ActionResult::RevealEachTopOfLibrary(source, self.clone()));
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
        results.push_settled(ActionResult::RevealEachTopOfLibrary(source, self.clone()));
    }
}
