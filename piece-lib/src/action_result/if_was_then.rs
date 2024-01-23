use crate::{
    action_result::Action,
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    log::Log,
    pending_results::PendingResults,
    player::Controller,
    protogen::{effects::Effect, targets::Restriction},
};

#[derive(Debug, Clone)]
pub(crate) struct IfWasThen {
    pub(crate) if_was: Vec<Restriction>,
    pub(crate) then: Vec<Effect>,
    pub(crate) source: CardId,
    pub(crate) controller: Controller,
}

impl Action for IfWasThen {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            if_was,
            then,
            source,
            controller,
        } = self;
        let mut results = PendingResults::default();
        let entries = Log::current_session(db);
        if entries
            .iter()
            .any(|entry| entry.1.left_battlefield_passes_restrictions(if_was))
        {
            for effect in then.iter() {
                effect.effect.as_ref().unwrap().push_pending_behavior(
                    db,
                    *source,
                    *controller,
                    &mut results,
                );
            }
        }

        results
    }
}
