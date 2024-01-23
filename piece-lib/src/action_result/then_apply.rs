use crate::{
    action_result::Action,
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    player::Controller,
};

#[derive(Debug, Clone)]
pub(crate) struct ThenApply {
    pub(crate) apply: Vec<crate::protogen::effects::Effect>,
    pub(crate) source: CardId,
    pub(crate) controller: Controller,
}

impl Action for ThenApply {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            apply,
            source,
            controller,
        } = self;
        let mut results = PendingResults::default();
        results.apply_in_stages();
        for effect in apply.iter() {
            effect.effect.as_ref().unwrap().push_pending_behavior(
                db,
                *source,
                *controller,
                &mut results,
            )
        }

        results
    }
}
