use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors},
    protogen,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub struct ForEach {
    pub restrictions: Vec<Restriction>,
    pub effects: Vec<Effect>,
    pub if_none: Vec<Effect>,
}

impl TryFrom<&protogen::effects::reveal_each_top_of_library::ForEach> for ForEach {
    type Error = anyhow::Error;

    fn try_from(
        value: &protogen::effects::reveal_each_top_of_library::ForEach,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
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

#[derive(Debug, Clone)]
pub struct RevealEachTopOfLibrary {
    pub for_each: ForEach,
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
    fn needs_targets(&self) -> usize {
        0
    }

    fn wants_targets(&self) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        _db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
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
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::RevealEachTopOfLibrary(source, self.clone()));
    }
}
