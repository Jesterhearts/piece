use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors},
    protogen,
    targets::Restriction,
};

#[derive(Debug)]
pub struct ApplyThenIfWas {
    apply: Vec<Effect>,
    then_if_was: Vec<Restriction>,
    then_apply: Vec<Effect>,
}

impl TryFrom<&protogen::effects::ApplyThenIfWas> for ApplyThenIfWas {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ApplyThenIfWas) -> Result<Self, Self::Error> {
        Ok(Self {
            apply: value
                .apply
                .iter()
                .map(Effect::try_from)
                .collect::<anyhow::Result<_>>()?,
            then_if_was: value
                .then
                .get_or_default()
                .if_was
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
            then_apply: value
                .then
                .get_or_default()
                .apply
                .iter()
                .map(Effect::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for ApplyThenIfWas {
    fn needs_targets(&'static self, db: &mut crate::in_play::Database) -> usize {
        self.apply
            .iter()
            .map(|effect| effect.needs_targets(db))
            .max()
            .unwrap()
    }

    fn wants_targets(&'static self, db: &mut crate::in_play::Database) -> usize {
        self.apply
            .iter()
            .map(|effect| effect.wants_targets(db))
            .max()
            .unwrap()
    }

    fn valid_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        self.apply
            .iter()
            .map(|effect| effect.valid_targets(db, source, controller, already_chosen))
            .max_by_key(|targets| targets.len())
            .unwrap()
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        for effect in self.apply.iter() {
            effect.push_pending_behavior(db, source, controller, results);
        }
    }

    fn push_behavior_with_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        for effect in self.apply.iter() {
            effect.push_behavior_with_targets(
                db,
                targets.clone(),
                apply_to_self,
                source,
                controller,
                results,
            );
        }
        results.push_settled(ActionResult::IfWasThen {
            if_was: self.then_if_was.clone(),
            then: self.then_apply.clone(),
            source,
            controller,
        })
    }
}
