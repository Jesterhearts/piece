use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ApplyAuraToTarget {
    pub(crate) aura_source: CardId,
    pub(crate) target: ActiveTarget,
}

impl Action for ApplyAuraToTarget {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            aura_source,
            target,
        } = self;

        match target {
            ActiveTarget::Battlefield { id } => {
                id.apply_aura(db, *aura_source);
            }
            ActiveTarget::Graveyard { .. } => todo!(),
            ActiveTarget::Player { .. } => todo!(),
            _ => unreachable!(),
        };
        PendingResults::default()
    }
}
