use crate::{
    effects::{Effect, EffectBehaviors},
    protogen,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct IfThenElse {
    if_: Vec<Restriction>,
    then: Box<Effect>,
    else_: Box<Effect>,
}

impl TryFrom<&protogen::effects::IfThenElse> for IfThenElse {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::IfThenElse) -> Result<Self, Self::Error> {
        Ok(Self {
            if_: value
                .if_
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
            then: Box::new(value.then.get_or_default().try_into()?),
            else_: Box::new(value.else_.get_or_default().try_into()?),
        })
    }
}

impl EffectBehaviors for IfThenElse {
    fn needs_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
    ) -> usize {
        if source.passes_restrictions(db, source, &self.if_) {
            self.then.needs_targets(db, source)
        } else {
            self.else_.needs_targets(db, source)
        }
    }

    fn wants_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
    ) -> usize {
        if source.passes_restrictions(db, source, &self.if_) {
            self.then.wants_targets(db, source)
        } else {
            self.else_.wants_targets(db, source)
        }
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        if source.passes_restrictions(db, source, &self.if_) {
            self.then
                .valid_targets(db, source, controller, already_chosen)
        } else {
            self.else_
                .valid_targets(db, source, controller, already_chosen)
        }
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if source.passes_restrictions(db, source, &self.if_) {
            self.then
                .push_pending_behavior(db, source, controller, results)
        } else {
            self.else_
                .push_pending_behavior(db, source, controller, results)
        }
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        if source.passes_restrictions(db, source, &self.if_) {
            self.then.push_behavior_with_targets(
                db,
                targets,
                apply_to_self,
                source,
                controller,
                results,
            )
        } else {
            self.else_.push_behavior_with_targets(
                db,
                targets,
                apply_to_self,
                source,
                controller,
                results,
            )
        }
    }
}
