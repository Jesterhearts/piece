use crate::{
    action_result::Action,
    in_play::{Database, ModifierId},
    pending_results::PendingResults,
    stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ModifyCreatures {
    pub(crate) targets: Vec<ActiveTarget>,
    pub(crate) modifier: ModifierId,
}

impl Action for ModifyCreatures {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { targets, modifier } = self;
        for target in targets {
            let target = match target {
                ActiveTarget::Battlefield { id } => id,
                ActiveTarget::Graveyard { id } => id,
                _ => unreachable!(),
            };
            target.apply_modifier(db, *modifier);
        }
        PendingResults::default()
    }
}
